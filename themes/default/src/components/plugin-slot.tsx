import { useEffect, useRef } from "react";
import { waitForNoteva } from "@/hooks/useNoteva";

interface PluginSlotProps {
  name: string;
  className?: string;
}

export function PluginSlot({ name, className }: PluginSlotProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let active = true;

    const renderSlot = async () => {
      const element = ref.current;
      if (!element) return;

      const sdk = await waitForNoteva({ timeout: 5_000 });
      if (!active || !sdk || !ref.current) return;

      sdk.slots.render(name, ref.current);
    };

    void renderSlot();

    return () => {
      active = false;
    };
  }, [name]);

  return <div ref={ref} data-noteva-slot={name} className={className} />;
}

export default PluginSlot;
