import { useEffect, useRef } from "react";

interface EscapeLayer {
  id: number;
  onEscape: () => void;
}

const escapeLayers: EscapeLayer[] = [];
let nextEscapeLayerId = 1;

export function useEscapeLayer(active: boolean, onEscape: () => void) {
  const idRef = useRef<number | null>(null);
  const onEscapeRef = useRef(onEscape);

  useEffect(() => {
    onEscapeRef.current = onEscape;
  }, [onEscape]);

  useEffect(() => {
    if (!active) return;

    const id = nextEscapeLayerId++;
    idRef.current = id;
    escapeLayers.push({
      id,
      onEscape: () => onEscapeRef.current(),
    });

    return () => {
      const index = escapeLayers.findIndex(layer => layer.id === id);
      if (index >= 0) {
        escapeLayers.splice(index, 1);
      }
      idRef.current = null;
    };
  }, [active]);
}

if (typeof window !== "undefined") {
  window.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    const topLayer = escapeLayers[escapeLayers.length - 1];
    if (!topLayer) return;
    event.preventDefault();
    event.stopPropagation();
    topLayer.onEscape();
  });
}
