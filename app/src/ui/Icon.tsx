// A Phosphor icon drawn in currentColor.

export default function Icon(props: { icon: string; size?: number; label?: string; class?: string }) {
  return (
    <svg
      class={`icon ${props.class ?? ""}`}
      width={props.size ?? 16}
      height={props.size ?? 16}
      viewBox="0 0 256 256"
      fill="currentColor"
      role={props.label ? "img" : undefined}
      aria-label={props.label}
      aria-hidden={props.label ? undefined : true}
      innerHTML={props.icon}
    />
  );
}
