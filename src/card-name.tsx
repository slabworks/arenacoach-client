import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { loadCardImage } from './card-image';

export default function CardName({ grpId, label }: { grpId: number; label: string }) {
    const [preview, setPreview] = useState<{ url: string; x: number; y: number } | null>(null);
    const generation = useRef(0);
    const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const trigger = useRef<HTMLButtonElement>(null);
    const id = useId();

    useEffect(() => () => {
        generation.current += 1;
        clearTimeout(timer.current);
    }, [grpId]);

    function hide() {
        generation.current += 1;
        clearTimeout(timer.current);
        setPreview(null);
    }

    function show() {
        clearTimeout(timer.current);
        const request = ++generation.current;
        timer.current = setTimeout(() => {
            void loadCardImage(grpId).then((url) => {
                if (!url || request !== generation.current || !trigger.current) return;
                const rect = trigger.current.getBoundingClientRect();
                setPreview({ url, x: Math.max(12, Math.min(rect.left, window.innerWidth - 256)), y: Math.max(12, Math.min(rect.bottom + 8, window.innerHeight - 352)) });
            }).catch(() => { if (request === generation.current) setPreview(null); });
        }, 120);
    }

    return <>
        <button ref={trigger} type="button" className="card-name cursor-help border-0 bg-transparent p-0 text-inherit underline decoration-dotted underline-offset-4"
            aria-describedby={preview ? id : undefined}
            onMouseEnter={show}
            onMouseLeave={() => { if (document.activeElement !== trigger.current) hide(); }}
            onFocus={show} onBlur={hide}
            onClick={() => { if (preview) hide(); else show(); }}
            onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); hide(); } }}>
            {label}
        </button>
        {preview ? createPortal(<div id={id} role="tooltip" className="card-image-float pointer-events-none fixed z-50 overflow-hidden rounded-xl border bg-black shadow-2xl"
            style={{ position: 'fixed', zIndex: 100, left: preview.x, top: preview.y, width: 'min(244px, calc(100vw - 24px))', maxHeight: 'calc(100vh - 24px)', pointerEvents: 'none' }}>
            <img src={preview.url} alt={label} width={244} height={340} style={{ display: 'block', width: '100%', height: 'auto', maxHeight: 'calc(100vh - 24px)', objectFit: 'contain' }} />
        </div>, document.body) : null}
    </>;
}
