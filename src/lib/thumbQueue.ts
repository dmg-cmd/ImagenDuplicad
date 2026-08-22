import { useEffect, useState } from "react";

const MAX_CONCURRENT = 4;

let active = 0;
const waiting: (() => void)[] = [];

export function enqueue<T>(fn: () => Promise<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    const run = () => {
      active++;
      fn()
        .then(resolve, reject)
        .finally(() => {
          active--;
          const next = waiting.shift();
          if (next) next();
        });
    };
    if (active < MAX_CONCURRENT) run();
    else waiting.push(run);
  });
}

export function useInView<T extends HTMLElement>() {
  const [node, setNode] = useState<T | null>(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    if (!node || inView) return;
    const obs = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) setInView(true);
      },
      { rootMargin: "300px" },
    );
    obs.observe(node);
    return () => obs.disconnect();
  }, [node, inView]);

  return { ref: setNode, inView } as const;
}
