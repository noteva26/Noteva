import { useEffect, useRef, useState } from "react";
import { ChevronDown, Music2, Pause, Play, RotateCcw, RotateCw } from "lucide-react";
import { waitForNoteva } from "@/hooks/useNoteva";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface MusicSettings {
  music_enabled?: unknown;
  music_label?: unknown;
  music_title?: unknown;
  music_artist?: unknown;
  music_cover?: unknown;
  music_src?: unknown;
}

function readString(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function readEnabled(value: unknown) {
  return value === true || value === "true" || value === "1";
}

function formatTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  const minutes = Math.floor(seconds / 60);
  const rest = Math.floor(seconds % 60);
  return `${minutes}:${String(rest).padStart(2, "0")}`;
}

export function MusicPlayer() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [settings, setSettings] = useState<MusicSettings | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);

  useEffect(() => {
    let active = true;

    const loadSettings = async () => {
      const Noteva = await waitForNoteva({ timeout: 3_000 });
      if (!active || !Noteva) return;

      try {
        const values = await Noteva.theme.getSettings();
        if (active) setSettings(values as MusicSettings);
      } catch {
        if (active) setSettings(null);
      }
    };

    void loadSettings();

    return () => {
      active = false;
    };
  }, []);

  const enabled = readEnabled(settings?.music_enabled);
  const src = readString(settings?.music_src);
  const title = readString(settings?.music_title) || "Untitled Track";
  const artist = readString(settings?.music_artist) || "Noteva";
  const label = readString(settings?.music_label) || "Now Playing";
  const cover = readString(settings?.music_cover);
  const progress = duration > 0 ? Math.min(100, (currentTime / duration) * 100) : 0;

  if (!enabled || !src) return null;

  const togglePlayback = async () => {
    const audio = audioRef.current;
    if (!audio) return;

    if (audio.paused) {
      await audio.play();
    } else {
      audio.pause();
    }
  };

  const seekBy = (seconds: number) => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.currentTime = Math.max(0, Math.min(audio.duration || 0, audio.currentTime + seconds));
  };

  return (
    <section
      className={cn(
        "noteva-music-player fixed bottom-4 right-4 z-40 overflow-hidden border bg-card/95 text-card-foreground shadow-2xl shadow-foreground/15 backdrop-blur supports-[backdrop-filter]:bg-card/88 sm:bottom-6 sm:right-6",
        expanded
          ? "w-[min(calc(100vw-2rem),22rem)] rounded-xl"
          : "w-[min(calc(100vw-2rem),16rem)] rounded-full"
      )}
    >
      {!expanded ? (
        <button
          type="button"
          className="flex w-full items-center gap-3 px-3 py-2 text-left"
          onClick={() => setExpanded(true)}
          aria-label="Expand music player"
        >
          <span className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-full border bg-muted">
            {cover ? (
              <img src={cover} alt="" className="h-full w-full object-cover" loading="lazy" />
            ) : (
              <Music2 className="h-5 w-5 text-muted-foreground" />
            )}
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-xs font-medium text-muted-foreground">{label}</span>
            <span className="block truncate text-sm font-semibold text-foreground">{title}</span>
          </span>
          <span
            className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground"
            onClick={(event) => {
              event.stopPropagation();
              void togglePlayback();
            }}
          >
            {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="ml-0.5 h-4 w-4" />}
          </span>
        </button>
      ) : (
        <>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="absolute right-2 top-2 z-10 h-8 w-8 rounded-full bg-background/70"
        aria-label="Collapse music player"
        onClick={() => setExpanded(false)}
      >
        <ChevronDown className="h-4 w-4" />
      </Button>
      <div className="border-b border-border bg-card/95 p-4 pb-5">
        <div className="flex items-center gap-4">
          {cover ? (
            <img
              src={cover}
              alt=""
              width={88}
              height={88}
              loading="lazy"
              className="h-16 w-16 flex-none rounded-lg border bg-muted object-cover sm:h-20 sm:w-20"
            />
          ) : (
            <div className="flex h-16 w-16 flex-none items-center justify-center rounded-lg border bg-muted sm:h-20 sm:w-20">
              <Play className="h-6 w-6 text-muted-foreground" />
            </div>
          )}
          <div className="min-w-0 flex-auto space-y-1 font-semibold">
            <p className="text-xs leading-5 text-primary sm:text-sm">{label}</p>
            <h2 className="truncate text-sm leading-6 text-muted-foreground">{artist}</h2>
            <p className="truncate text-base text-foreground sm:text-lg">{title}</p>
          </div>
        </div>
        <div className="mt-4 space-y-2">
          <div className="relative">
            <div className="overflow-hidden rounded-full bg-muted">
              <div
                className="h-2 bg-primary"
                role="progressbar"
                aria-label="music progress"
                aria-valuenow={Math.round(currentTime)}
                aria-valuemin={0}
                aria-valuemax={Math.round(duration || 0)}
                style={{ width: `${progress}%` }}
              />
            </div>
            <div
              className="absolute top-1/2 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded-full bg-background shadow ring-2 ring-primary"
              style={{ left: `calc(${progress}% - 0.5rem)` }}
            >
              <div className="h-1.5 w-1.5 rounded-full bg-primary ring-1 ring-inset ring-foreground/5" />
            </div>
          </div>
          <div className="flex justify-between text-sm font-medium leading-6 tabular-nums">
            <div className="text-primary">{formatTime(currentTime)}</div>
            <div className="text-muted-foreground">{formatTime(duration)}</div>
          </div>
        </div>
      </div>
      <div className="flex items-center bg-muted/65 text-muted-foreground">
        <div className="flex flex-auto items-center justify-evenly">
          <Button type="button" variant="ghost" size="icon" aria-label="Rewind 10 seconds" onClick={() => seekBy(-10)}>
            <RotateCcw className="h-5 w-5" />
          </Button>
        </div>
        <Button
          type="button"
          className={cn(
            "-my-2 mx-auto h-16 w-16 flex-none rounded-full bg-background text-foreground shadow-md ring-1 ring-foreground/5 hover:bg-background sm:h-20 sm:w-20"
          )}
          aria-label={isPlaying ? "Pause" : "Play"}
          onClick={() => void togglePlayback()}
        >
          {isPlaying ? <Pause className="h-7 w-7 sm:h-8 sm:w-8" /> : <Play className="ml-1 h-7 w-7 sm:h-8 sm:w-8" />}
        </Button>
        <div className="flex flex-auto items-center justify-evenly">
          <Button type="button" variant="ghost" size="icon" aria-label="Skip 10 seconds" onClick={() => seekBy(10)}>
            <RotateCw className="h-5 w-5" />
          </Button>
        </div>
      </div>
      </>
      )}
      <audio
        ref={audioRef}
        src={src}
        preload="metadata"
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onEnded={() => setIsPlaying(false)}
        onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime)}
        onLoadedMetadata={(event) => setDuration(event.currentTarget.duration || 0)}
      />
    </section>
  );
}
