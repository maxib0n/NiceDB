export const presentation = $state({ on: false });

export function togglePresentation(on = !presentation.on) {
  presentation.on = on;
  if (on) document.documentElement.dataset.presentation = '';
  else delete document.documentElement.dataset.presentation;
}
