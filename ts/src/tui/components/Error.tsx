import { TextAttributes } from "@opentui/core";

interface ErrorProps {
  error: string;
}

export function Error({ error }: ErrorProps) {
  return (
    <box alignItems="center" justifyContent="center" flexGrow={1} gap={1}>
      <text attributes={TextAttributes.BOLD}>Frost failed to load config</text>
      <text attributes={TextAttributes.DIM}>{error}</text>
    </box>
  );
}
