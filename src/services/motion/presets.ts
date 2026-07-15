type SpringTransition = {
  type: "spring";
  stiffness: number;
  damping: number;
  mass: number;
};

export const springSnappy: SpringTransition = {
  type: "spring",
  stiffness: 520,
  damping: 38,
  mass: 0.82,
};

export const springSoft: SpringTransition = {
  type: "spring",
  stiffness: 380,
  damping: 34,
  mass: 0.92,
};

export const dockReveal = {
  initial: { opacity: 0, y: 10, scale: 0.97 },
  animate: { opacity: 1, y: 0, scale: 1 },
  exit: { opacity: 0, y: 8, scale: 0.98 },
  transition: springSoft,
};

export const threadReveal = {
  initial: {
    opacity: 0,
    y: 28,
    scaleY: 0.72,
    transformOrigin: "bottom center",
  },
  animate: {
    opacity: 1,
    y: 0,
    scaleY: 1,
    transformOrigin: "bottom center",
  },
  exit: {
    opacity: 0,
    y: 18,
    scaleY: 0.84,
    transformOrigin: "bottom center",
  },
  transition: {
    ...springSoft,
    opacity: { duration: 0.18 },
  },
};

export const messageReveal = {
  initial: { opacity: 0, y: 12, scale: 0.98 },
  animate: { opacity: 1, y: 0, scale: 1 },
  exit: { opacity: 0, y: 6, scale: 0.99 },
  transition: springSnappy,
};

export const controlsReveal = {
  initial: { opacity: 0, x: 8, scale: 0.92 },
  animate: { opacity: 1, x: 0, scale: 1 },
  exit: { opacity: 0, x: 6, scale: 0.94 },
  transition: springSnappy,
};

export const suggestionsReveal = {
  initial: { opacity: 0, height: 0 },
  animate: { opacity: 1, height: "auto" },
  exit: { opacity: 0, height: 0 },
  transition: { duration: 0.22, ease: [0.22, 1, 0.36, 1] },
};

export const suggestionItemReveal = {
  initial: { opacity: 0, x: -6 },
  animate: { opacity: 1, x: 0 },
  exit: { opacity: 0, x: -4 },
  transition: springSnappy,
};

export const typingPulse = {
  animate: { opacity: [0.35, 1, 0.35] },
  transition: { duration: 1.35, repeat: Infinity, ease: "easeInOut" },
};

export function staggerDelay(index: number, step = 0.04) {
  return { delay: index * step };
}
