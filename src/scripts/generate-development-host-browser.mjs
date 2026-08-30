import { cpSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const engine = resolve(process.env.RUSTY_ENGINE_ROOT ?? join(root, '..', 'rusty-engine'));
const hostModule = join(engine, 'render', 'artifacts', 'product-browser-host', 'product-browser-host.js');
const { productBrowserBundleAssets } = await import(`data:text/javascript;base64,${Buffer.from(await (await import('node:fs/promises')).readFile(hostModule)).toString('base64')}`);
const output = join(root, 'src', 'RustyD20.NativeProduct', 'DevelopmentHost', 'generated-browser');
for (const asset of productBrowserBundleAssets({ engineHostModule: await (await import('node:fs/promises')).readFile(hostModule, 'utf8'), uiModule: './ui/main.js', runtimeAdapterModule: './runtime-adapter.js', lifecycleMode: 'demand', uiProjection: { expectedStream: 'rusty-d20', expectedContract: 'rusty-d20.workbench.v1' } })) { const destination = join(output, asset.name); mkdirSync(dirname(destination), { recursive: true }); writeFileSync(destination, asset.content); }
mkdirSync(join(output, 'ui'), { recursive: true }); cpSync(join(root, 'src', 'RustyD20.NativeProduct', 'DevelopmentHost', 'browser-source', 'ui', 'main.js'), join(output, 'ui', 'main.js')); cpSync(join(root, 'src', 'RustyD20.NativeProduct', 'DevelopmentHost', 'browser-source', 'runtime-adapter.js'), join(output, 'runtime-adapter.js'));
