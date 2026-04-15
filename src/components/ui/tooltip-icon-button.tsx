import { useState, type ButtonHTMLAttributes, type ReactNode } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip";

type TooltipSide = "top" | "right" | "bottom" | "left";
type TooltipAlign = "start" | "center" | "end";

type TooltipIconButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
  tooltipSide?: TooltipSide;
  tooltipAlign?: TooltipAlign;
  tooltipSideOffset?: number;
  tooltipClassName?: string;
  delay?: number;
};

export function TooltipIconButton({
  label,
  children,
  tooltipSide = "bottom",
  tooltipAlign = "center",
  tooltipSideOffset = 6,
  tooltipClassName,
  delay = 200,
  type = "button",
  title,
  "aria-label": ariaLabel,
  onMouseEnter,
  onMouseLeave,
  onFocus,
  onBlur,
  ...buttonProps
}: TooltipIconButtonProps) {
  const [open, setOpen] = useState(false);

  return (
    <Tooltip open={open}>
      <TooltipTrigger
        render={<button />}
        delay={delay}
        type={type}
        title={title ?? label}
        aria-label={ariaLabel ?? label}
        onMouseEnter={(event) => {
          onMouseEnter?.(event);
          if (!buttonProps.disabled) {
            setOpen(true);
          }
        }}
        onMouseLeave={(event) => {
          onMouseLeave?.(event);
          setOpen(false);
        }}
        onFocus={(event) => {
          onFocus?.(event);
          if (!buttonProps.disabled) {
            setOpen(true);
          }
        }}
        onBlur={(event) => {
          onBlur?.(event);
          setOpen(false);
        }}
        {...buttonProps}
      >
        {children}
      </TooltipTrigger>
      <TooltipContent
        role="tooltip"
        side={tooltipSide}
        align={tooltipAlign}
        sideOffset={tooltipSideOffset}
        className={tooltipClassName}
      >
        {label}
      </TooltipContent>
    </Tooltip>
  );
}
