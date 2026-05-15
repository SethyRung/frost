import type { ProcessStatus } from "@/process/types";

interface StatusIconProps {
  status: ProcessStatus;
}

export function StatusIcon({ status }: StatusIconProps) {
  let icon: string;
  switch (status) {
    case "running":
      icon = "\u25CF";
      break;
    case "starting":
    case "stopping":
      icon = "\u25D4";
      break;
    case "crashed":
      icon = "\u2717";
      break;
    case "stopped":
    default:
      icon = "\u25CB";
      break;
  }

  return <text>{icon}</text>;
}
