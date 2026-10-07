export function helpHost(
  open: boolean,
  onclose: () => void = () => {},
): { open: boolean; onclose: () => void } {
  const props = $state({ open, onclose });
  return props;
}
