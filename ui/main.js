const campaignControls = [
  ['Warden\'s Gate', 'd20.select.warden'], ['Ember\'s Wake', 'd20.select.ember'], ['Begin', 'd20.begin'],
  ['Forward (W)', 'd20.forward'], ['Back (S)', 'd20.back'], ['Turn left (A)', 'd20.left'], ['Turn right (D)', 'd20.right'], ['Interact (E)', 'd20.interact'],
  ['Save', 'd20.save'], ['Load', 'd20.load'], ['Reset', 'd20.reset'],
];
const tacticalControls = [
  ['Move north', 'd20.tactical.move.north'], ['Move south', 'd20.tactical.move.south'], ['Move west', 'd20.tactical.move.west'], ['Move east', 'd20.tactical.move.east'],
  ['Party', 'd20.party.next'], ['Action', 'd20.action.next'], ['Target', 'd20.target.next'], ['Commit action', 'd20.action.commit'], ['React', 'd20.reaction.choose'], ['Decline (Esc)', 'd20.reaction.decline'],
];
const outcomeControls = [['Continue', 'd20.outcome.continue']];

export function mountProductUi(root, context) {
  const panel = document.createElement('section'); panel.setAttribute('aria-label', 'Rusty D20 controls and observations'); panel.style.maxWidth = '72rem';
  const background = document.createElement('div'); background.setAttribute('data-d20-modal-background', 'true'); panel.append(background);
  const title = document.createElement('h1'); title.textContent = 'Rusty D20'; title.tabIndex = -1; background.append(title);
  const status = document.createElement('output'); status.setAttribute('aria-live', 'polite'); status.textContent = 'Waiting for Engine projection…'; background.append(status);
  const buttons = document.createElement('div'); buttons.setAttribute('aria-label', 'Campaign controls'); buttons.style.cssText = 'display:flex;flex-wrap:wrap;gap:.5rem';
  const emit = (intent) => context.intents?.claim(intent, { kind: 'digital', active: true });
  const addControls = (container, definitions) => definitions.map(([label, intent]) => { const button = document.createElement('button'); button.type = 'button'; button.textContent = label; button.dataset.intent = intent; button.style.minHeight = '44px'; button.addEventListener('click', () => emit(intent)); container.append(button); return button; });
  const campaignButtons = addControls(buttons, campaignControls); background.append(buttons);
  const outcome = document.createElement('section'); outcome.setAttribute('aria-label', 'Outcome controls'); outcome.hidden = true;
  const outcomeButtons = addControls(outcome, outcomeControls); background.append(outcome);
  const observations = document.createElement('section'); observations.setAttribute('aria-label', 'Campaign observer fields');
  const observationTitle = document.createElement('h2'); observationTitle.textContent = 'Campaign observer'; observations.append(observationTitle);
  const observerList = document.createElement('dl'); observerList.style.cssText = 'display:grid;grid-template-columns:max-content 1fr;gap:.35rem 1rem'; observations.append(observerList); background.append(observations);
  const tactical = document.createElement('section'); tactical.setAttribute('role', 'dialog'); tactical.setAttribute('aria-modal', 'true'); tactical.setAttribute('aria-label', 'Tactical workbench observer'); tactical.tabIndex = -1; tactical.hidden = true; tactical.style.cssText = 'border:2px solid #8a6510;padding:1rem;margin-top:1rem;background:#17130b';
  const tacticalTitle = document.createElement('h2'); tacticalTitle.textContent = 'Tactical board — Engine scene; observer readout'; tactical.append(tacticalTitle);
  const tacticalButtonGroup = document.createElement('div'); tacticalButtonGroup.setAttribute('aria-label', 'Tactical controls'); tacticalButtonGroup.style.cssText = 'display:flex;flex-wrap:wrap;gap:.5rem'; const modalButtons = addControls(tacticalButtonGroup, tacticalControls); tactical.append(tacticalButtonGroup);
  const tacticalList = document.createElement('dl'); tacticalList.style.cssText = 'display:grid;grid-template-columns:max-content 1fr;gap:.35rem 1rem'; tactical.append(tacticalList); panel.append(tactical);
  const logSection = document.createElement('section'); const logTitle = document.createElement('h2'); logTitle.textContent = 'Resolution log'; logSection.append(logTitle); const log = document.createElement('ol'); log.setAttribute('aria-live', 'polite'); logSection.append(log); background.append(logSection); root.append(panel);
  const renderFields = (list, fields) => list.replaceChildren(...fields.flatMap(([key, value]) => { const term = document.createElement('dt'); term.textContent = key; const detail = document.createElement('dd'); detail.textContent = String(value ?? ''); return [term, detail]; }));
  const focusable = () => Array.from(tactical.querySelectorAll('button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'));
  let modalActive = false; let previousFocus = null;
  const enterTacticalModal = () => {
    if (modalActive) return;
    modalActive = true; previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    tactical.hidden = false; background.inert = true; background.setAttribute('aria-hidden', 'true'); campaignButtons.forEach((button) => { button.disabled = true; });
    queueMicrotask(() => (focusable()[0] ?? tactical).focus());
  };
  const exitTacticalModal = () => {
    if (!modalActive) return;
    modalActive = false; tactical.hidden = true; background.inert = false; background.removeAttribute('aria-hidden'); campaignButtons.forEach((button) => { button.disabled = false; });
    (previousFocus?.isConnected && !previousFocus.disabled ? previousFocus : title).focus(); previousFocus = null;
  };
  const render = (envelope) => {
    const value = envelope?.value; if (!value || typeof value !== 'object') return;
    const phase = value['campaign.phase'] ?? 'Camp'; status.textContent = `${phase} — ${value['readout.last'] ?? 'ready'}`;
    const entries = Object.entries(value).filter(([key]) => !key.startsWith('log.'));
    renderFields(observerList, entries.filter(([key]) => !key.startsWith('tactical.') && !key.startsWith('selection.')));
    if (phase === 'Encounter') { enterTacticalModal(); renderFields(tacticalList, entries); } else exitTacticalModal();
    const outcomeActive = phase === 'Outcome'; outcome.hidden = !outcomeActive; outcomeButtons.forEach((button) => { button.disabled = !outcomeActive; });
    log.replaceChildren(...Object.entries(value).filter(([key]) => key.startsWith('log.')).map(([, text]) => { const item = document.createElement('li'); item.textContent = String(text); return item; }));
  };
  const onKeydown = (event) => {
    if (modalActive && event.key === 'Tab') { const candidates = focusable(); const index = candidates.indexOf(document.activeElement); if (candidates.length === 0) { event.preventDefault(); tactical.focus(); } else if (event.shiftKey && index <= 0) { event.preventDefault(); candidates.at(-1).focus(); } else if (!event.shiftKey && index === candidates.length - 1) { event.preventDefault(); candidates[0].focus(); } return; }
    const intent = (modalActive ? { ArrowUp: 'd20.tactical.move.north', ArrowDown: 'd20.tactical.move.south', ArrowLeft: 'd20.tactical.move.west', ArrowRight: 'd20.tactical.move.east', Escape: 'd20.reaction.decline' } : { w: 'd20.forward', s: 'd20.back', a: 'd20.left', d: 'd20.right', e: 'd20.interact' })[event.key];
    if (intent) { event.preventDefault(); emit(intent); }
  };
  globalThis.addEventListener('keydown', onKeydown);
  render(context.projection?.current() ?? null); const unsubscribe = context.projection?.subscribe(render) ?? (() => {}); return { dispose() { exitTacticalModal(); unsubscribe(); globalThis.removeEventListener('keydown', onKeydown); } };
}
