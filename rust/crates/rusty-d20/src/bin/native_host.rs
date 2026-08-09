use std::{
    collections::BTreeSet,
    env,
    io::{self, Write},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use rusty_d20::{
    presentation::create_game_frame, ExplorationCommandKindDto, ExplorationCommandRequestDto,
    GameRuntime, GameSnapshotDto, NewAdventureRequestDto,
};
use rusty_engine::{
    render_host_contracts::{
        RendererCameraPose, RendererCameraProjection, RendererCompositionCamera,
        RendererCompositionTarget, RendererPickFilter, RendererPickRay, RendererPickRequest,
        RendererTargetColor, RendererTargetDepth, RendererTargetSampling, RendererViewComposition,
        RendererViewTarget, RendererViewport, RENDERER_VIEW_COMPOSITION_SCHEMA_VERSION,
    },
    render_model::{RenderHandle, RenderLayer},
    renderer_webview_host::{
        RendererWebviewAdapter, RendererWebviewBounds, RendererWebviewObservation,
        RendererWebviewOptions,
    },
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(Debug, Clone, Copy)]
struct Options {
    proof: bool,
}

impl Options {
    fn parse() -> Result<Self> {
        let mut proof = false;
        for argument in env::args().skip(1) {
            match argument.as_str() {
                "--proof" => proof = true,
                _ => bail!("unknown argument {argument}"),
            }
        }
        Ok(Self { proof })
    }
}

#[derive(Debug, Default)]
struct Proof {
    frame: bool,
    views: bool,
    camera: bool,
    resize: bool,
    input_authority: bool,
    input_noop: bool,
    pick_authority: bool,
    pick_miss: bool,
    state: bool,
    render: bool,
    save_round_trip: bool,
}

impl Proof {
    fn complete(&self) -> bool {
        self.frame
            && self.views
            && self.camera
            && self.resize
            && self.input_authority
            && self.input_noop
            && self.pick_authority
            && self.pick_miss
            && self.state
            && self.render
            && self.save_round_trip
    }
}

#[derive(Debug, Clone, Copy)]
enum PickKind {
    Miss,
    Control,
}

#[derive(Debug, Clone, Copy)]
struct PendingPick {
    request_id: u64,
    kind: PickKind,
    revision_before: u64,
}

struct NativeApplication {
    options: Options,
    runtime: GameRuntime,
    window: Option<Window>,
    renderer: Option<RendererWebviewAdapter>,
    retained_handles: Vec<RenderHandle>,
    published_revision: Option<u64>,
    pending_frame: Option<(u64, u64, Vec<RenderHandle>)>,
    pending_input: Option<u64>,
    pending_pick: Option<PendingPick>,
    dispose_request: Option<u64>,
    previous_pressed_codes: BTreeSet<String>,
    next_input_poll: Instant,
    started_at: Instant,
    ready: bool,
    proof: Proof,
    failure: Option<String>,
}

impl NativeApplication {
    fn new(options: Options) -> Result<Self> {
        let mut runtime = GameRuntime::empty().context("create empty D20 runtime")?;
        let catalog = runtime.snapshot().context("project D20 catalog")?;
        let adventure_id = catalog
            .available_adventures
            .first()
            .context("D20 catalog contains no selectable adventure")?
            .id
            .clone();
        let camp = runtime
            .new_adventure_for(NewAdventureRequestDto {
                expected_revision: catalog.revision,
                adventure_id,
            })
            .context("start checked D20 adventure")?;
        runtime
            .begin_exploration(camp.revision)
            .context("enter checked D20 dungeon")?;
        Ok(Self {
            options,
            runtime,
            window: None,
            renderer: None,
            retained_handles: Vec::new(),
            published_revision: None,
            pending_frame: None,
            pending_input: None,
            pending_pick: None,
            dispose_request: None,
            previous_pressed_codes: BTreeSet::new(),
            next_input_poll: Instant::now(),
            started_at: Instant::now(),
            ready: false,
            proof: Proof::default(),
            failure: None,
        })
    }

    fn mount(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Rusty D20 — preparing native renderer")
                    .with_inner_size(winit::dpi::LogicalSize::new(960, 640)),
            )
            .context("create Rusty D20 product window")?;
        let renderer = RendererWebviewAdapter::mount(
            &window,
            RendererWebviewOptions {
                auto_start: true,
                bounds: window_bounds(&window),
                clear_color: Some(0x090d12),
                pixel_ratio: window.scale_factor(),
                resources: Vec::new(),
            },
        )
        .map_err(|error| anyhow::anyhow!("mount Engine-owned renderer: {error:?}"))?;
        self.window = Some(window);
        self.renderer = Some(renderer);
        Ok(())
    }

    fn initialize_renderer(&mut self) -> Result<()> {
        let snapshot = self.runtime.snapshot()?;
        let frame = create_game_frame(&snapshot, &self.retained_handles)
            .map_err(|error| anyhow::anyhow!("project D20 retained frame: {error:?}"))?;
        let renderer = self.renderer.as_mut().context("renderer unavailable")?;
        self.pending_frame = Some((
            renderer.submit_frame(&frame.frame)?,
            snapshot.revision,
            frame.handles,
        ));
        renderer.configure_views(&product_views(1))?;
        renderer.set_camera_pose(frame.camera, None)?;
        renderer.read_state()?;
        renderer.render_once(None)?;
        let bounds = window_bounds(self.window.as_ref().context("window unavailable")?);
        renderer.resize(
            RendererWebviewBounds {
                width: bounds.width.saturating_sub(48).max(1),
                height: bounds.height.saturating_sub(32).max(1),
                ..bounds
            },
            self.window
                .as_ref()
                .context("window unavailable")?
                .scale_factor(),
        )?;
        self.request_input()?;
        self.update_title(&snapshot);
        Ok(())
    }

    fn publish_runtime(&mut self) -> Result<()> {
        if self.pending_frame.is_some() {
            return Ok(());
        }
        let snapshot = self.runtime.snapshot()?;
        if self.published_revision == Some(snapshot.revision) {
            return Ok(());
        }
        let frame = create_game_frame(&snapshot, &self.retained_handles)
            .map_err(|error| anyhow::anyhow!("project D20 retained frame: {error:?}"))?;
        let renderer = self.renderer.as_mut().context("renderer unavailable")?;
        let request_id = renderer.submit_frame(&frame.frame)?;
        renderer.set_camera_pose(frame.camera, None)?;
        self.pending_frame = Some((request_id, snapshot.revision, frame.handles));
        self.update_title(&snapshot);
        Ok(())
    }

    fn update_title(&self, snapshot: &GameSnapshotDto) {
        if let Some(window) = &self.window {
            let phase = snapshot
                .campaign
                .as_ref()
                .map(|campaign| format!("{:?}", campaign.phase))
                .unwrap_or_else(|| "Catalog".to_owned());
            window.set_title(&format!(
                "Rusty D20 — native Engine renderer — {phase} r{}",
                snapshot.revision
            ));
        }
    }

    fn request_input(&mut self) -> Result<()> {
        if self.pending_input.is_none() {
            self.pending_input = Some(
                self.renderer
                    .as_mut()
                    .context("renderer unavailable")?
                    .read_physical_input()?,
            );
        }
        Ok(())
    }

    fn apply_input(
        &mut self,
        input: &rusty_engine::render_host_contracts::RendererPhysicalInputReadout,
    ) -> Result<()> {
        let pressed = input.pressed_codes.iter().cloned().collect::<BTreeSet<_>>();
        if let Some(code) = pressed
            .difference(&self.previous_pressed_codes)
            .next()
            .cloned()
        {
            let revision_before = self.runtime.snapshot()?.revision;
            match code.as_str() {
                "Enter" => {
                    self.command(ExplorationCommandKindDto::TurnRight)?;
                    let revision_after = self.runtime.snapshot()?.revision;
                    self.proof.input_authority = revision_after > revision_before;
                    if self.pending_pick.is_none() {
                        self.request_pick(PickKind::Miss)?;
                    }
                }
                "Escape" => {
                    self.proof.input_noop = self.runtime.snapshot()?.revision == revision_before;
                }
                "ArrowLeft" => self.command(ExplorationCommandKindDto::TurnLeft)?,
                "ArrowRight" => self.command(ExplorationCommandKindDto::TurnRight)?,
                "ArrowUp" => self.command(ExplorationCommandKindDto::StepForward)?,
                "ArrowDown" => self.command(ExplorationCommandKindDto::StepBackward)?,
                _ => {}
            }
        }
        self.previous_pressed_codes = pressed;
        Ok(())
    }

    fn command(&mut self, command: ExplorationCommandKindDto) -> Result<()> {
        let revision = self.runtime.snapshot()?.revision;
        self.runtime
            .exploration_command(ExplorationCommandRequestDto {
                expected_revision: revision,
                command,
            })?;
        self.publish_runtime()
    }

    fn request_pick(&mut self, kind: PickKind) -> Result<()> {
        let ray = match kind {
            PickKind::Miss => RendererPickRay::WorldRay {
                origin: [1000.0, 10.0, 1000.0],
                direction: [0.0, -1.0, 0.0],
            },
            PickKind::Control => RendererPickRay::WorldRay {
                origin: [0.0, 5.0, -1.0],
                direction: [0.0, -1.0, 0.0],
            },
        };
        let revision_before = self.runtime.snapshot()?.revision;
        let request_id = self
            .renderer
            .as_mut()
            .context("renderer unavailable")?
            .pick(&RendererPickRequest {
                filter: Some(RendererPickFilter {
                    layers: vec![RenderLayer::Scene],
                    tags: vec!["native-control".to_owned()],
                    ..RendererPickFilter::default()
                }),
                max_distance: Some(32.0),
                ray,
            })?;
        self.pending_pick = Some(PendingPick {
            request_id,
            kind,
            revision_before,
        });
        Ok(())
    }

    fn apply_pick(
        &mut self,
        request_id: u64,
        receipt: rusty_engine::render_host_contracts::RendererPickReceipt,
    ) -> Result<()> {
        let pending = self
            .pending_pick
            .take()
            .context("unexpected pick receipt")?;
        if pending.request_id != request_id {
            bail!(
                "pick request mismatch: received {request_id}, expected {}",
                pending.request_id
            );
        }
        match pending.kind {
            PickKind::Miss => {
                if receipt.hint.is_some()
                    || self.runtime.snapshot()?.revision != pending.revision_before
                {
                    bail!("miss pick changed D20 authority");
                }
                self.proof.pick_miss = true;
                self.request_pick(PickKind::Control)?;
            }
            PickKind::Control => {
                let hint = receipt
                    .hint
                    .context("native control pick returned no hit")?;
                if !hint.tags.iter().any(|tag| tag == "native-control") {
                    bail!(
                        "native control pick returned unrelated tags: {:?}",
                        hint.tags
                    );
                }
                self.command(ExplorationCommandKindDto::TurnLeft)?;
                self.proof.pick_authority =
                    self.runtime.snapshot()?.revision > pending.revision_before;
                let encoded = self.runtime.encode_save()?;
                let restored = GameRuntime::decode_save(&encoded)?;
                self.proof.save_round_trip = restored.encode_save()? == encoded;
            }
        }
        Ok(())
    }

    fn handle_observation(
        &mut self,
        observation: RendererWebviewObservation,
        event_loop: &ActiveEventLoop,
    ) -> Result<()> {
        match observation {
            RendererWebviewObservation::Ready(_) => {
                if self.options.proof {
                    println!("RUSTY_D20_NATIVE_READY_FOR_INPUT");
                    io::stdout().flush()?;
                }
                self.ready = true;
                self.initialize_renderer()?;
            }
            RendererWebviewObservation::FrameApplied {
                request_id,
                receipt,
            } => {
                if !receipt.applied {
                    bail!("renderer rejected D20 frame: {:?}", receipt.diagnostics);
                }
                if let Some((pending_id, revision, handles)) = self.pending_frame.take() {
                    if request_id != pending_id {
                        bail!(
                            "frame request mismatch: received {request_id}, expected {pending_id}"
                        );
                    }
                    self.retained_handles = handles;
                    self.published_revision = Some(revision);
                }
                self.proof.frame = true;
                self.publish_runtime()?;
            }
            RendererWebviewObservation::ViewsConfigured { receipt, .. } => {
                if !receipt.applied {
                    bail!("renderer rejected D20 views: {:?}", receipt.diagnostics);
                }
                self.proof.views = true;
            }
            RendererWebviewObservation::CameraUpdated { .. } => self.proof.camera = true,
            RendererWebviewObservation::PhysicalInputRead {
                request_id,
                readout,
            } if self.pending_input == Some(request_id) => {
                self.pending_input = None;
                self.apply_input(&readout)?;
            }
            RendererWebviewObservation::PickCompleted {
                request_id,
                receipt,
            } => self.apply_pick(request_id, receipt)?,
            RendererWebviewObservation::StateRead { .. } => self.proof.state = true,
            RendererWebviewObservation::FrameRendered { .. } => self.proof.render = true,
            RendererWebviewObservation::Resized { .. } => self.proof.resize = true,
            RendererWebviewObservation::Disposed { request_id }
                if self.dispose_request == Some(request_id) =>
            {
                println!(
                    "RUSTY_D20_NATIVE_PROOF_OK frame={} views={} camera={} resize={} input_authority={} input_noop={} pick_authority={} pick_miss={} state={} render={} save_round_trip={} lifecycle=disposed",
                    self.proof.frame,
                    self.proof.views,
                    self.proof.camera,
                    self.proof.resize,
                    self.proof.input_authority,
                    self.proof.input_noop,
                    self.proof.pick_authority,
                    self.proof.pick_miss,
                    self.proof.state,
                    self.proof.render,
                    self.proof.save_round_trip,
                );
                event_loop.exit();
            }
            RendererWebviewObservation::MountFailed { message } => {
                self.renderer = None;
                bail!("renderer mount failed transactionally: {message}");
            }
            RendererWebviewObservation::OperationFailed {
                request_id,
                operation,
                message,
            } => bail!("renderer operation {operation:?} request {request_id} failed: {message}"),
            _ => {}
        }
        Ok(())
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl std::fmt::Display) {
        self.renderer = None;
        self.failure = Some(error.to_string());
        event_loop.exit();
    }
}

impl ApplicationHandler for NativeApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(error) = self.mount(event_loop) {
                self.fail(event_loop, error);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) && self.dispose_request.is_none() {
            match self.renderer.as_mut().map(RendererWebviewAdapter::dispose) {
                Some(Ok(request_id)) => self.dispose_request = Some(request_id),
                Some(Err(error)) => self.fail(event_loop, error),
                None => event_loop.exit(),
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
        if self.options.proof && self.started_at.elapsed() > Duration::from_secs(40) {
            self.fail(
                event_loop,
                format!("native renderer proof timed out: {:?}", self.proof),
            );
            return;
        }
        let observations = self
            .renderer
            .as_mut()
            .map(RendererWebviewAdapter::drain_observations)
            .unwrap_or_default();
        for observation in observations {
            let result = observation
                .map_err(anyhow::Error::from)
                .and_then(|observation| self.handle_observation(observation, event_loop));
            if let Err(error) = result {
                self.fail(event_loop, error);
                return;
            }
        }
        if self.failure.is_some() || self.dispose_request.is_some() {
            return;
        }
        if self.ready && self.renderer.is_some() && Instant::now() >= self.next_input_poll {
            if let Err(error) = self.request_input() {
                self.fail(event_loop, error);
                return;
            }
            self.next_input_poll = Instant::now() + Duration::from_millis(40);
        }
        if self.options.proof && self.proof.complete() {
            match self.renderer.as_mut().map(RendererWebviewAdapter::dispose) {
                Some(Ok(request_id)) => self.dispose_request = Some(request_id),
                Some(Err(error)) => self.fail(event_loop, error),
                None => self.fail(event_loop, "renderer disappeared before disposal"),
            }
        }
    }
}

fn product_views(target_revision: u64) -> RendererViewComposition {
    RendererViewComposition {
        schema_version: RENDERER_VIEW_COMPOSITION_SCHEMA_VERSION,
        cameras: vec![RendererCompositionCamera {
            id: "camera.rusty-d20".to_owned(),
            pose: RendererCameraPose {
                position: [0.0, 12.0, 0.0],
                pitch_degrees: -90.0,
                yaw_degrees: 0.0,
            },
            projection: RendererCameraProjection::Orthographic {
                vertical_size: 18.0,
                near: 0.1,
                far: 32.0,
            },
        }],
        targets: vec![RendererCompositionTarget {
            id: "target.rusty-d20".to_owned(),
            revision: target_revision,
            width: 256,
            height: 256,
            color: RendererTargetColor::Rgba8Srgb,
            depth: RendererTargetDepth::Depth24,
            sampling: RendererTargetSampling::Nearest,
        }],
        views: vec![
            rusty_engine::render_host_contracts::RendererCompositionView {
                id: "view.rusty-d20".to_owned(),
                camera_id: "camera.rusty-d20".to_owned(),
                target: RendererViewTarget::Offscreen {
                    target_id: "target.rusty-d20".to_owned(),
                    target_revision,
                },
                viewport: RendererViewport {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                },
                order: 10,
            },
        ],
        presentations: Vec::new(),
    }
}

fn window_bounds(window: &Window) -> RendererWebviewBounds {
    let size = window.inner_size();
    let scale = window.scale_factor();
    RendererWebviewBounds {
        x: 0,
        y: 0,
        width: ((f64::from(size.width) / scale).round() as u32).max(1),
        height: ((f64::from(size.height) / scale).round() as u32).max(1),
    }
}

fn main() -> Result<()> {
    #[cfg(target_os = "linux")]
    gtk::init().context("initialize GTK for native renderer host")?;
    let options = Options::parse()?;
    let event_loop = EventLoop::new().context("create Rusty D20 event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut application = NativeApplication::new(options)?;
    event_loop
        .run_app(&mut application)
        .context("run Rusty D20 native product")?;
    if let Some(failure) = application.failure {
        bail!(failure);
    }
    Ok(())
}
