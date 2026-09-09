import { useEffect, useRef, useState, type MouseEvent } from "react";
import { createPortal } from "react-dom";
import { loadCardImage } from "./card-image";

export default function CardName({
  grpId,
  label,
}: {
  grpId: number;
  label: string;
}) {
  const [preview, setPreview] = useState<{
    url: string;
    x: number;
    y: number;
  } | null>(null);
  const [hovering, setHovering] = useState(false);
  const timer = useRef<number>(0);
  const request = useRef(0);
  const node = useRef<HTMLSpanElement>(null);
  const cursor = useRef({ x: 0, y: 0 });

  function hide(): void {
    request.current += 1;
    setHovering(false);
    window.clearTimeout(timer.current);
    setPreview(null);
  }

  useEffect(() => {
    return () => window.clearTimeout(timer.current);
  }, []);

  useEffect(() => {
    if (!hovering) {
      return;
    }

    function pointerStillOnName(): boolean {
      const el = node.current;
      if (!el) {
        return false;
      }

      const under = document.elementFromPoint(
        cursor.current.x,
        cursor.current.y,
      );
      return Boolean(under && (under === el || el.contains(under)));
    }

    function hideIfPointerLeft(event?: Event): void {
      if (event && "clientX" in event && "clientY" in event) {
        cursor.current = {
          x: (event as WheelEvent).clientX,
          y: (event as WheelEvent).clientY,
        };
      }

      if (!pointerStillOnName()) {
        hide();
      }
    }

    window.addEventListener("scroll", hideIfPointerLeft, true);
    window.addEventListener("wheel", hideIfPointerLeft, {
      capture: true,
      passive: true,
    });
    window.addEventListener("blur", hide);

    return () => {
      window.removeEventListener("scroll", hideIfPointerLeft, true);
      window.removeEventListener("wheel", hideIfPointerLeft, true);
      window.removeEventListener("blur", hide);
    };
  }, [hovering]);

  function follow(event: MouseEvent<HTMLElement>): void {
    cursor.current = { x: event.clientX, y: event.clientY };
    setPreview((current) =>
      current === null ? null : { ...current, ...cursor.current },
    );
  }

  function showPreview(event: MouseEvent<HTMLElement>): void {
    cursor.current = { x: event.clientX, y: event.clientY };
    setHovering(true);
    window.clearTimeout(timer.current);
    const id = ++request.current;
    timer.current = window.setTimeout(() => {
      void loadCardImage(grpId).then((url) => {
        if (!url || request.current !== id) {
          return;
        }

        setPreview({ url, ...cursor.current });
      });
    }, 120);
  }

  return (
    <>
      <span
        ref={node}
        className="card-name"
        onMouseEnter={showPreview}
        onMouseMove={follow}
        onMouseLeave={hide}
      >
        {label}
      </span>
      {preview ? (
        <CardImageFloat url={preview.url} x={preview.x} y={preview.y} />
      ) : null}
    </>
  );
}

function CardImageFloat({
  url,
  x,
  y,
}: {
  url: string;
  x: number;
  y: number;
}) {
  const width = 244;
  const height = 340;
  const left = Math.min(x + 16, window.innerWidth - width - 12);
  const top = Math.min(y + 16, window.innerHeight - height - 12);

  return createPortal(
    <div className="card-image-float" style={{ left, top, width }}>
      <img src={url} alt="" width={width} height={height} />
    </div>,
    document.body,
  );
}
