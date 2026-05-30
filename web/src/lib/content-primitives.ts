function formatDate(value: string, locale?: string, timezone?: string) {
  const raw = value.trim();
  if (!raw) return "";

  const date = new Date(raw.length === 10 ? `${raw}T00:00:00` : raw);
  if (Number.isNaN(date.getTime())) return raw;

  const options: Intl.DateTimeFormatOptions = {
    year: "numeric",
    month: "short",
    day: "numeric",
  };

  if (raw.includes("T")) {
    options.hour = "2-digit";
    options.minute = "2-digit";
  }

  if (timezone) {
    options.timeZone = timezone;
  }

  try {
    return new Intl.DateTimeFormat(locale || undefined, options).format(date);
  } catch {
    return raw;
  }
}

export function enhanceRenderedContentPrimitives(root: ParentNode, locale?: string) {
  const revealSpoiler = (element: HTMLElement) => {
    element.classList.add("is-revealed");
    element.setAttribute("aria-expanded", "true");
  };

  root.querySelectorAll<HTMLElement>(".noteva-spoiler:not([data-noteva-bound])").forEach((element) => {
    element.dataset.notevaBound = "1";
    element.addEventListener("click", () => revealSpoiler(element));
    element.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        revealSpoiler(element);
      }
    });
  });

  root.querySelectorAll<HTMLTimeElement>(".noteva-date[data-noteva-date]").forEach((element) => {
    if (element.dataset.notevaFormattedLocale === locale) return;

    const value = element.dataset.notevaRawValue || element.getAttribute("datetime") || element.textContent || "";
    element.dataset.notevaRawValue = value;
    element.textContent = formatDate(value, locale, element.dataset.timezone);
    element.dataset.notevaFormattedLocale = locale || "";
  });

  root.querySelectorAll<HTMLElement>(".noteva-date-range[data-noteva-date-range]").forEach((element) => {
    if (element.dataset.notevaFormattedLocale === locale) return;

    const from = element.dataset.from || "";
    const to = element.dataset.to || "";
    const formattedFrom = formatDate(from, locale, element.dataset.timezone);
    const formattedTo = formatDate(to, locale, element.dataset.timezone);

    if (formattedFrom && formattedTo) {
      element.textContent = `${formattedFrom} - ${formattedTo}`;
      element.dataset.notevaFormattedLocale = locale || "";
    }
  });
}
