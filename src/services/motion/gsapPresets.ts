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

const COMPOSER_ENTER = 0.18;
const COMPOSER_LEAVE = 0.14;
const OVERLAY_ENTER = 0.28;
const OVERLAY_LEAVE = 0.16;

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
    duration: opts?.duration ?? 0.32,
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
        duration: 0.14,
        stagger: 0.016,
        clearProps: "transform",
      },
      0.04,
    );
  }
}

export function gsapPickerLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
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
    { autoAlpha: 0, y: 4 },
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

  // Prefer opacity over autoAlpha here: visibility:hidden prevents focusing the
  // input when the overlay pops up (focus races with the enter tween start state).
  // Horizontal expand uses clip-path (not scaleX) so text/input aren't stretched.
  if (prefersReducedMotion()) {
    gsap.set(target, {
      opacity: visible ? 1 : 0,
      visibility: visible ? "visible" : "hidden",
      clearProps: visible ? "opacity,clipPath,visibility" : undefined,
    });
    return;
  }

  if (visible) {
    gsap.set(target, {
      visibility: "visible",
      opacity: 1,
      clipPath: "inset(0 50% 0 50% round 8px)",
    });
    gsap.to(target, {
      clipPath: "inset(0 0% 0 0% round 8px)",
      duration: OVERLAY_ENTER,
      ease: "power3.out",
      clearProps: "clipPath",
    });
    return;
  }

  gsap.to(target, {
    clipPath: "inset(0 50% 0 50% round 8px)",
    duration: OVERLAY_LEAVE,
    ease: "power2.in",
    onComplete: () => {
      gsap.set(target, {
        visibility: "hidden",
        clearProps: "clipPath",
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
    { autoAlpha: 0, y: 10 },
    {
      autoAlpha: 1,
      y: 0,
      duration: 0.22,
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
    duration: 0.12,
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
    { autoAlpha: 0, x: -8 },
    {
      autoAlpha: 1,
      x: 0,
      duration: 0.2,
      stagger: 0.028,
      clearProps: "transform",
    },
  );
}
