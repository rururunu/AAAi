import gsap from "gsap";

function prefersReducedMotion() {
  return typeof window !== "undefined"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Ask / approval / permission picker panel.
 * Do NOT tween container height — Overlay resizes the window for that.
 * Height tweens here fight the window resize and flash the message panel.
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

  gsap.fromTo(
    target,
    { opacity: 0 },
    {
      opacity: 1,
      duration: 0.14,
      ease: "power2.out",
      onComplete: done,
    },
  );

  if (items.length) {
    gsap.fromTo(
      items,
      { opacity: 0, x: -4 },
      {
        opacity: 1,
        x: 0,
        duration: 0.12,
        stagger: 0.016,
        ease: "power2.out",
        clearProps: "transform",
      },
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
    opacity: 0,
    duration: 0.1,
    ease: "power2.in",
    onComplete: done,
  });
}

/** Floating model / approval mode menu (call after position is applied). */
export function gsapMenuPrepare(el: Element) {
  gsap.set(el as HTMLElement, { opacity: 0 });
}

export function gsapMenuEnter(el: Element, done?: () => void) {
  const target = el as HTMLElement;
  if (prefersReducedMotion()) {
    gsap.set(target, { opacity: 1, y: 0, scale: 1, clearProps: "transform" });
    done?.();
    return;
  }

  gsap.killTweensOf(target);
  gsap.fromTo(
    target,
    { opacity: 0, y: 5, scale: 0.98 },
    {
      opacity: 1,
      y: 0,
      scale: 1,
      duration: 0.14,
      ease: "power2.out",
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
  gsap.to(target, {
    opacity: 0,
    y: 3,
    scale: 0.98,
    duration: 0.1,
    ease: "power2.in",
    onComplete: done,
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
    { opacity: 0, y: 10 },
    {
      opacity: 1,
      y: 0,
      duration: 0.22,
      ease: "power2.out",
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
    opacity: 0,
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
    { opacity: 0, x: -8 },
    {
      opacity: 1,
      x: 0,
      duration: 0.2,
      stagger: 0.028,
      ease: "power2.out",
      clearProps: "transform",
    },
  );
}
