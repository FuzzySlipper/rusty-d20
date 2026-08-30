const controls = [
  ['Warden\'s Gate', 'd20.select.warden'], ['Ember\'s Wake', 'd20.select.ember'], ['Begin', 'd20.begin'],
  ['Forward (W)', 'd20.forward'], ['Back (S)', 'd20.back'], ['Turn left (A)', 'd20.left'], ['Turn right (D)', 'd20.right'], ['Interact (E)', 'd20.interact'],
  ['Party', 'd20.party.next'], ['Action', 'd20.action.next'], ['Target', 'd20.target.next'], ['Commit action', 'd20.action.commit'],
  ['React', 'd20.reaction.choose'], ['Decline (Esc)', 'd20.reaction.decline'], ['Continue', 'd20.outcome.continue'], ['Save', 'd20.save'], ['Load', 'd20.load'], ['Reset', 'd20.reset'],
];

export function mountProductUi(root, context) {
  const panel = document.createElement('section'); panel.setAttribute('aria-label', 'Rusty D20 controls and observations');
  const title = document.createElement('h1'); title.textContent = 'Rusty D20'; panel.append(title);
  const status = document.createElement('output'); status.setAttribute('aria-live', 'polite'); status.textContent = 'Waiting for Engine projection…'; panel.append(status);
  const buttons = document.createElement('div'); buttons.style.cssText = 'display:flex;flex-wrap:wrap;gap:.5rem';
  const emit = (intent) => context.intents?.claim(intent, { kind: 'digital', active: true });
  for (const [label, intent] of controls) { const button = document.createElement('button'); button.type = 'button'; button.textContent = label; button.style.minHeight = '44px'; button.addEventListener('click', () => emit(intent)); buttons.append(button); }
  panel.append(buttons);
  const log = document.createElement('ol'); log.setAttribute('aria-live', 'polite'); panel.append(log); root.append(panel);
  const render = (envelope) => { const value = envelope?.value; if (!value || typeof value !== 'object') return; status.textContent = `${value['campaign.phase'] ?? 'Camp'} — ${value['readout.last'] ?? 'ready'}`; log.replaceChildren(...Object.entries(value).filter(([key]) => key.startsWith('log.')).map(([, text]) => { const item = document.createElement('li'); item.textContent = String(text); return item; })); };
  const onKeydown = (event) => { const intent = ({ w: 'd20.forward', s: 'd20.back', a: 'd20.left', d: 'd20.right', e: 'd20.interact', Escape: 'd20.reaction.decline' })[event.key]; if (intent) { event.preventDefault(); emit(intent); } };
  globalThis.addEventListener('keydown', onKeydown);
  render(context.projection?.current() ?? null); const unsubscribe = context.projection?.subscribe(render) ?? (() => {}); return { dispose() { unsubscribe(); globalThis.removeEventListener('keydown', onKeydown); } };
}
