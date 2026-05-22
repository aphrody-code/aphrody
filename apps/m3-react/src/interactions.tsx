// SPDX-License-Identifier: Apache-2.0
//! Interaction primitives distilled from design.google and gemini.google.com.
//
// Grounded in a live runtime analysis (2026-05-23) of the two surfaces:
//
//   design.google — restrained scroll: sticky header, every <img loading=lazy>,
//     and navigation through the **View Transitions API** (`view-transition-name:
//     root`). Cards do NOT opacity/transform-reveal; hover is a 0.165s color
//     transition on a decelerate curve `cubic-bezier(0, 0.4, 0.2, 1)`. Reveals,
//     when used, are IntersectionObserver-driven (no CSS scroll timelines).
//
//   gemini.google.com — the "long reflection" (thinking) state is built from
//     `gem-shimmer-sweep` (skeleton shimmer), `animateGradient`/`gradientScroll`
//     (the moving brand gradient), `input-area-spin` (an animated gradient ring
//     around the composer while processing), and a 4-colour circular spinner.
//     Answers stream in token-by-token and each block `fade-in-up`s on arrival.
//
// These React 19 hooks/components reproduce those behaviours on standard
// platform APIs, all reduced-motion aware.

import {useEffect, useRef, useState} from 'react';
import type {CSSProperties, MouseEvent as ReactMouseEvent, ReactNode} from 'react';

// ── shared ────────────────────────────────────────────────────────────────

/** design.google's decelerate hover curve. */
const DECELERATE = 'cubic-bezier(0, 0.4, 0.2, 1)';
/** M3 emphasized decelerate (entry). */
const EMPHASIZED_DECELERATE = 'cubic-bezier(0.05, 0.7, 0.1, 1)';
/** Gemini brand gradient stops. */
const BRAND_GRADIENT = 'linear-gradient(90deg, #4285f4, #9168c0 55%, #d96570)';

function prefersReducedMotion(): boolean {
  return (
    typeof matchMedia === 'function' &&
    matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

/** Inject a keyframes/style block once, by id. */
function useInjectedStyles(id: string, css: string): void {
  useEffect(() => {
    if (typeof document === 'undefined' || document.getElementById(id)) {
      return;
    }
    const el = document.createElement('style');
    el.id = id;
    el.textContent = css;
    document.head.appendChild(el);
  }, [id, css]);
}

// ── View Transitions (design.google navigation) ────────────────────────────

/**
 * Run a DOM/state update inside a View Transition (the mechanism design.google
 * uses for navigation, `view-transition-name: root`). Falls back to a plain
 * synchronous update when the API is unavailable or the user prefers reduced
 * motion.
 */
export function startViewTransition(update: () => void | Promise<void>): void {
  if (
    !prefersReducedMotion() &&
    typeof document !== 'undefined' &&
    'startViewTransition' in document
  ) {
    document.startViewTransition(update);
  } else {
    void update();
  }
}

/**
 * Returns a wrapper that runs a React state transition inside a view
 * transition, for smooth route/content swaps.
 */
export function useViewTransition(): (update: () => void) => void {
  return startViewTransition;
}

interface ViewTransitionLinkProps {
  href: string;
  /** SPA navigation handler; when omitted the link sets `location.href`. */
  onNavigate?: (href: string) => void;
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
}

/**
 * An anchor that navigates inside a View Transition — the design.google click
 * behaviour. For SPA routers, pass `onNavigate`; otherwise it performs a full
 * navigation (still transitioned for same-document SPA frameworks).
 */
export function ViewTransitionLink({
  href,
  onNavigate,
  children,
  className,
  style,
}: ViewTransitionLinkProps) {
  const handleClick = (event: ReactMouseEvent<HTMLAnchorElement>) => {
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) {
      return; // let the browser handle new-tab / modified clicks
    }
    event.preventDefault();
    startViewTransition(() => {
      if (onNavigate) {
        onNavigate(href);
      } else {
        location.href = href;
      }
    });
  };
  return (
    <a
      href={href}
      onClick={handleClick}
      className={className}
      style={{transition: `color 0.165s ${DECELERATE}`, ...style}}
    >
      {children}
    </a>
  );
}

// ── Scroll reveal (IntersectionObserver, design.google style) ───────────────

interface RevealProps {
  children: ReactNode;
  /** px to translate up from on entry. */
  distance?: number;
  /** ms transition duration. */
  duration?: number;
  /** stagger delay (ms) for sequenced lists. */
  delay?: number;
  className?: string;
  style?: CSSProperties;
}

/**
 * Fades + lifts its children into view the first time they intersect the
 * viewport — IntersectionObserver-driven (no CSS scroll timeline), matching how
 * Google's surfaces reveal content. No-ops to visible under reduced motion.
 */
export function Reveal({
  children,
  distance = 16,
  duration = 500,
  delay = 0,
  className,
  style,
}: RevealProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [shown, setShown] = useState(false);

  useEffect(() => {
    const node = ref.current;
    if (!node || prefersReducedMotion() || typeof IntersectionObserver === 'undefined') {
      setShown(true);
      return;
    }
    const io = new IntersectionObserver(
      (entries, obs) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setShown(true);
            obs.disconnect();
          }
        }
      },
      {rootMargin: '0px 0px -10% 0px', threshold: 0.05},
    );
    io.observe(node);
    return () => io.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      className={className}
      style={{
        opacity: shown ? 1 : 0,
        transform: shown ? 'none' : `translateY(${distance}px)`,
        transition: `opacity ${duration}ms ${EMPHASIZED_DECELERATE} ${delay}ms, transform ${duration}ms ${EMPHASIZED_DECELERATE} ${delay}ms`,
        willChange: 'opacity, transform',
        ...style,
      }}
    >
      {children}
    </div>
  );
}

// ── Lazy image with fade-in (design.google) ─────────────────────────────────

interface LazyImageProps {
  src: string;
  alt: string;
  className?: string;
  style?: CSSProperties;
}

/** A natively-lazy image that fades in once decoded (design.google pattern). */
export function LazyImage({src, alt, className, style}: LazyImageProps) {
  const [loaded, setLoaded] = useState(false);
  return (
    <img
      src={src}
      alt={alt}
      loading="lazy"
      decoding="async"
      onLoad={() => setLoaded(true)}
      className={className}
      style={{
        opacity: loaded || prefersReducedMotion() ? 1 : 0,
        transition: `opacity 400ms ${DECELERATE}`,
        ...style,
      }}
    />
  );
}

// ── Gemini "long reflection" — thinking indicator ───────────────────────────

interface ThinkingIndicatorProps {
  /** Optional status label, e.g. "Réflexion en cours…". */
  label?: string;
  className?: string;
}

/**
 * Reproduces Gemini's thinking affordance: a shimmer sweep over a brand-gradient
 * bar (`gem-shimmer-sweep` + `animateGradient`). Use while awaiting the first
 * streamed token. Static under reduced motion.
 */
export function ThinkingIndicator({label, className}: ThinkingIndicatorProps) {
  useInjectedStyles(
    'aphrody-thinking-kf',
    `@keyframes aphrody-shimmer-sweep{0%{background-position:200% 0}100%{background-position:-200% 0}}
     @keyframes aphrody-gradient-scroll{0%,100%{background-position:0% 50%}50%{background-position:100% 50%}}
     @media (prefers-reduced-motion: reduce){
       .aphrody-thinking-bar,.aphrody-thinking-dot{animation:none!important}
     }`,
  );
  return (
    <div
      className={className}
      role="status"
      aria-live="polite"
      aria-label={label ?? 'Réflexion en cours'}
      style={{display: 'flex', alignItems: 'center', gap: 10}}
    >
      <span
        className="aphrody-thinking-dot"
        style={{
          width: 18,
          height: 18,
          borderRadius: 9999,
          background: BRAND_GRADIENT,
          backgroundSize: '200% 200%',
          animation: 'aphrody-gradient-scroll 2s ease-in-out infinite',
        }}
      />
      <span
        className="aphrody-thinking-bar"
        style={{
          flex: 1,
          maxWidth: 220,
          height: 12,
          borderRadius: 9999,
          background:
            'linear-gradient(90deg, var(--md-sys-color-surface-container-high, #2d2d2d) 25%, var(--md-sys-color-surface-container-highest, #3c4043) 50%, var(--md-sys-color-surface-container-high, #2d2d2d) 75%)',
          backgroundSize: '200% 100%',
          animation: 'aphrody-shimmer-sweep 1.4s linear infinite',
        }}
      />
      {label ? (
        <span style={{fontSize: 13, color: 'var(--md-sys-color-on-surface-variant, #c4c7c5)'}}>
          {label}
        </span>
      ) : null}
    </div>
  );
}

// ── Gemini composer "processing" gradient ring (input-area-spin) ────────────

interface GradientBorderProps {
  children: ReactNode;
  /** When true, the brand-gradient border animates (Gemini "thinking" ring). */
  active?: boolean;
  radius?: number | string;
  thickness?: number;
  className?: string;
  style?: CSSProperties;
}

/**
 * Wraps content in an animated brand-gradient border — Gemini's `input-area-spin`
 * ring shown around the composer while a response is generating. When inactive
 * the border is a static neutral outline.
 */
export function GradientBorder({
  children,
  active = false,
  radius = 28,
  thickness = 2,
  className,
  style,
}: GradientBorderProps) {
  useInjectedStyles(
    'aphrody-gradient-border-kf',
    `@keyframes aphrody-border-spin{0%,100%{background-position:0% 50%}50%{background-position:100% 50%}}
     @media (prefers-reduced-motion: reduce){.aphrody-gb-active{animation:none!important}}`,
  );
  const r = typeof radius === 'number' ? `${radius}px` : radius;
  return (
    <div
      className={`${active ? 'aphrody-gb-active' : ''} ${className ?? ''}`}
      style={{
        padding: thickness,
        borderRadius: r,
        background: active
          ? BRAND_GRADIENT
          : 'var(--md-sys-color-outline-variant, #c4c7c5)',
        backgroundSize: active ? '300% 300%' : undefined,
        animation: active ? 'aphrody-border-spin 3s ease-in-out infinite' : undefined,
        ...style,
      }}
    >
      <div
        style={{
          borderRadius: `calc(${r} - ${thickness}px)`,
          background: 'var(--md-sys-color-surface, #1f1f1f)',
          height: '100%',
        }}
      >
        {children}
      </div>
    </div>
  );
}

// ── Streaming text (token-by-token reveal, fade-in-up) ──────────────────────

interface StreamingTextProps {
  /** The accumulated text so far (caller appends streamed chunks). */
  text: string;
  /** True once the stream is complete (hides the caret). */
  done?: boolean;
  className?: string;
  style?: CSSProperties;
}

/**
 * Renders progressively-streamed text with a blinking caret while in flight —
 * Gemini's answer reveal. The caller updates `text` as chunks arrive from the
 * `StreamGenerate` backend (see `crates/gemini-web`); newly-appended text
 * inherits a subtle fade. Caret hidden under reduced motion and when `done`.
 */
export function StreamingText({text, done = false, className, style}: StreamingTextProps) {
  useInjectedStyles(
    'aphrody-stream-kf',
    `@keyframes aphrody-caret-blink{0%,49%{opacity:1}50%,100%{opacity:0}}
     @keyframes aphrody-fade-in-up{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
     @media (prefers-reduced-motion: reduce){.aphrody-caret{animation:none}}`,
  );
  return (
    <span
      className={className}
      style={{
        animation: prefersReducedMotion() ? undefined : 'aphrody-fade-in-up 200ms ease-out',
        ...style,
      }}
    >
      {text}
      {!done ? (
        <span
          className="aphrody-caret"
          aria-hidden="true"
          style={{
            display: 'inline-block',
            width: '0.6ch',
            marginInlineStart: 1,
            background: 'currentColor',
            animation: 'aphrody-caret-blink 1s steps(1) infinite',
          }}
        >
          &#8203;|
        </span>
      ) : null}
    </span>
  );
}

/**
 * Accumulates chunks from an async iterable (e.g. a `StreamGenerate` reader)
 * into state for {@link StreamingText}. Returns the accumulated text and a
 * `done` flag.
 *
 * @example
 * const {text, done} = useStreamingText(stream);
 * return <StreamingText text={text} done={done} />;
 */
export function useStreamingText(
  stream: AsyncIterable<string> | undefined,
): {text: string; done: boolean} {
  const [text, setText] = useState('');
  const [done, setDone] = useState(false);
  useEffect(() => {
    if (!stream) {
      return;
    }
    let cancelled = false;
    setText('');
    setDone(false);
    (async () => {
      try {
        for await (const chunk of stream) {
          if (cancelled) {
            return;
          }
          setText((prev) => prev + chunk);
        }
      } finally {
        if (!cancelled) {
          setDone(true);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [stream]);
  return {text, done};
}
