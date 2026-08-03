import gsap from "gsap";
import { ScrollToPlugin } from "gsap/ScrollToPlugin";

gsap.registerPlugin(ScrollToPlugin);

gsap.defaults({
  ease: "power2.out",
});

function prefersReducedMotion() {
  return (
    typeof window !== "undefined"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

const COMPOSER_ENTER = 0.14;
const COMPOSER_LEAVE = 0.09;
const OVERLAY_ENTER = 0.17;
const OVERLAY_LEAVE = 0.1;

/**
 * Scroll a specific overflow container to a child element.
 * Prefer this over element.scrollIntoView — that can scroll the wrong
 * ancestor (window / overlay) in Peek's nested scroll layout.
 */
export function gsapScrollContainerTo(
  container: HTMLElement,
  target: HTMLElement,
  opts?: { offsetY?: number; duration?: number },
) {
  const offsetY = opts?.offsetY ?? 12;
  gsap.killTweensOf(container);

  if (prefersReducedMotion()) {
    const next =
      container.scrollTop
      + (target.getBoundingClientRect().top - container.getBoundingClientRect().top)
      - offsetY;
    container.scrollTop = Math.max(0, next);
    return;
  }

  gsap.to(container, {
    duration: opts?.duration ?? 0.22,
    scrollTo: { y: target, offsetY },
  });
}

/**
 * Ask / approval / permission picker panel.
 * Do NOT tween container height — Overlay resizes the window for that.
 * Height tweens here fight the window resize and flash the message panel.
 *
 * Composer floating menus: autoAlpha + x only, never drive window resize.
 */
export function gsapPickerEnter(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    gsap.set(target, { clearProps: "all" });
    done();
    return;
  }

  const items = target.querySelectorAll<HTMLElement>(
    ".command-item, .workspace-option, .workspace-new-row",
  );
  gsap.killTweensOf([target, ...items]);

  const tl = gsap.timeline({ onComplete: done });
  tl.fromTo(
    target,
    { autoAlpha: 0 },
    { autoAlpha: 1, duration: COMPOSER_ENTER },
  );

  if (items.length) {
    tl.fromTo(
      items,
      { autoAlpha: 0, x: -4 },
      {
        autoAlpha: 1,
        x: 0,
        duration: 0.11,
        stagger: 0.012,
        clearProps: "transform",
      },
      0.04,
    );
  }
}

export function gsapPickerLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (target.matches(".model-picker-list, .option-picker-list")) {
    const items = target.querySelectorAll<HTMLElement>(".command-item");
    gsap.killTweensOf([target, ...items]);
    done();
    return;
  }
  if (prefersReducedMotion()) {
    done();
    return;
  }

  gsap.killTweensOf(target);
  gsap.to(target, {
    autoAlpha: 0,
    duration: COMPOSER_LEAVE,
    ease: "power2.in",
    onComplete: done,
  });
}

/** Floating model / approval mode menu (call after position is applied). */
export function gsapMenuPrepare(el: Element) {
  gsap.set(el as HTMLElement, { autoAlpha: 0 });
}

export function gsapMenuEnter(el: Element, done?: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    gsap.set(target, { autoAlpha: 1, clearProps: "transform" });
    done?.();
    return;
  }

  gsap.killTweensOf(target);
  gsap.fromTo(
    target,
    { autoAlpha: 0, y: 3 },
    {
      autoAlpha: 1,
      y: 0,
      duration: COMPOSER_ENTER,
      clearProps: "transform",
      onComplete: () => done?.(),
    },
  );
}

export function gsapMenuLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    done();
    return;
  }

  gsap.killTweensOf(target);
  // Opacity-only leave — scale fights window resize and feels toy-like.
  gsap.to(target, {
    autoAlpha: 0,
    duration: COMPOSER_LEAVE,
    ease: "power2.in",
    onComplete: done,
  });
}

/**
 * Chat thread panel enter/leave.
 * Prefer opacity + y — never scaleY (fights overlay window resize).
 * Do not use autoAlpha: visibility:hidden at enter start hides the first
 * messages if the tween is interrupted by window resize.
 */
export function gsapOverlayThreadEnter(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    gsap.set(target, { clearProps: "all" });
    done();
    return;
  }

  gsap.killTweensOf(target);
  gsap.set(target, { visibility: "visible", opacity: 0, y: 10 });
  gsap.to(target, {
    opacity: 1,
    y: 0,
    duration: OVERLAY_ENTER,
    ease: "power3.out",
    clearProps: "transform,opacity",
    onComplete: done,
    onInterrupt: () => {
      gsap.set(target, { clearProps: "transform,opacity,visibility" });
      done();
    },
  });
}

export function gsapOverlayThreadLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    done();
    return;
  }

  gsap.killTweensOf(target);
  gsap.to(target, {
    opacity: 0,
    y: 8,
    duration: OVERLAY_LEAVE,
    ease: "power2.in",
    onComplete: done,
    onInterrupt: done,
  });
}

/** Composer dock show/hide when overlay visibility flips. */
export function gsapOverlayDockReveal(el: Element | null, visible: boolean) {
  if (!el) return;
  const target = el as HTMLElement;
  gsap.killTweensOf(target);

  // Keep the input focusable throughout the reveal. A short opacity/translate
  // tween avoids clip-path repaints and reaches an interactive frame sooner.
  if (prefersReducedMotion()) {
    gsap.set(target, {
      opacity: visible ? 1 : 0,
      visibility: visible ? "visible" : "hidden",
      clearProps: visible ? "opacity,transform,visibility" : undefined,
    });
    return;
  }

  if (visible) {
    gsap.set(target, {
      visibility: "visible",
      opacity: 0,
      y: 5,
    });
    gsap.to(target, {
      opacity: 1,
      y: 0,
      duration: OVERLAY_ENTER,
      ease: "power3.out",
      clearProps: "opacity,transform",
    });
    return;
  }

  gsap.to(target, {
    opacity: 0,
    y: 3,
    duration: OVERLAY_LEAVE,
    ease: "power2.in",
    onComplete: () => {
      gsap.set(target, {
        visibility: "hidden",
        clearProps: "opacity,transform",
      });
    },
  });
}

/** Settings category panel swap (right pane). */
export function gsapSettingsPanelEnter(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    gsap.set(target, { clearProps: "all" });
    done();
    return;
  }

  gsap.killTweensOf(target);
  gsap.fromTo(
    target,
    { autoAlpha: 0, y: 6 },
    {
      autoAlpha: 1,
      y: 0,
      duration: 0.17,
      clearProps: "all",
      onComplete: done,
      onInterrupt: done,
    },
  );
}

export function gsapSettingsPanelLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    done();
    return;
  }

  gsap.killTweensOf(target);
  gsap.to(target, {
    autoAlpha: 0,
    y: -6,
    duration: 0.09,
    ease: "power2.in",
    onComplete: done,
  });
}

/** Settings sidebar items on first open. */
export function gsapSettingsNavMount(root: Element) {
  if (prefersReducedMotion()) return;
  const items = root.querySelectorAll<HTMLElement>(".settings-nav-item");
  if (!items.length) return;
  gsap.killTweensOf(items);
  gsap.fromTo(
    items,
    { autoAlpha: 0, x: -5 },
    {
      autoAlpha: 1,
      x: 0,
      duration: 0.16,
      stagger: 0.018,
      clearProps: "transform",
    },
  );
}

/**
 * First-run welcome: move the floating logo onto the empty-conversation brand,
 * then circular-reveal the workspace by shrinking the overlay mask.
 */
export function gsapOnboardingReveal(opts: {
  overlay: HTMLElement;
  logo: HTMLElement;
  from: DOMRect;
  target: DOMRect;
  onComplete: () => void;
}) {
  const { overlay, logo, from, target, onComplete } = opts;
  const originX = target.left + target.width / 2;
  const originY = target.top + target.height / 2;

  gsap.killTweensOf([overlay, logo]);

  // Freeze the logo in viewport space so CSS layout changes cannot skew the path.
  gsap.set(logo, {
    position: "fixed",
    left: from.left,
    top: from.top,
    width: from.width,
    height: from.height,
    margin: 0,
    x: 0,
    y: 0,
    scale: 1,
    transformOrigin: "50% 50%",
    zIndex: 3,
  });

  if (prefersReducedMotion()) {
    onComplete();
    return;
  }

  const maxRadius = Math.hypot(
    Math.max(originX, window.innerWidth - originX),
    Math.max(originY, window.innerHeight - originY),
  );

  const state = { radius: maxRadius * 1.2 };
  overlay.style.clipPath = `circle(${state.radius}px at ${originX}px ${originY}px)`;

  const tl = gsap.timeline({
    onComplete: () => {
      overlay.style.clipPath = "";
      onComplete();
    },
  });

  tl.to(logo, {
    left: target.left,
    top: target.top,
    width: target.width,
    height: target.height,
    duration: 1.05,
    ease: "power3.inOut",
  });

  tl.to(
    state,
    {
      radius: 0,
      duration: 0.95,
      ease: "power2.inOut",
      onUpdate: () => {
        overlay.style.clipPath = `circle(${state.radius}px at ${originX}px ${originY}px)`;
      },
    },
    "-=0.28",
  );
}
