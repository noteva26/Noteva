import {
    useEffect,
    useRef,
    useCallback,
    forwardRef,
    useImperativeHandle,
    useState,
    useMemo,
    lazy,
    Suspense,
} from "react";
import { createPortal } from "react-dom";
import { EditorView, keymap, placeholder as cmPlaceholder, ViewUpdate } from "@codemirror/view";
import { EditorState, Compartment } from "@codemirror/state";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import { defaultKeymap, history, historyKeymap, undo, redo } from "@codemirror/commands";
import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
import { api, uploadApi, filesApi, type FileInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Loader2 } from "lucide-react";
import { useI18nStore, type Locale, t as i18nT } from "@/lib/i18n";
import { toast } from "sonner";

const EmojiPicker = lazy(() =>
    import("@/components/ui/emoji-picker").then((module) => ({
        default: module.EmojiPicker,
    }))
);

// Resolve plugin label: supports string or { "zh-CN": "...", "en": "..." } object
function resolveLabel(label: string | Record<string, string>, locale: Locale): string {
    if (typeof label === "string") return label;
    return label[locale] || label["en"] || label["zh-CN"] || Object.values(label)[0] || "";
}

export interface PluginEditorButton {
    id: string;
    label: string | Record<string, string>;
    icon?: string;
    insertBefore: string;
    insertAfter: string;
}

export interface MarkdownEditorRef {
    getValue: () => string;
    setValue: (value: string) => void;
    insertValue: (value: string, render?: boolean) => void;
    focus: () => void;
}

interface MarkdownEditorProps {
    initialValue?: string;
    onChange?: (value: string) => void;
    pluginButtons?: PluginEditorButton[];
    placeholder?: string;
    minHeight?: number;
}

// ── Toolbar button definitions ──────────────────────────────
// Inline SVG icons (16×16) for toolbar - keeps bundle small
const icons = {
    heading: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M6 12h12" /><path d="M6 4v16" /><path d="M18 4v16" /></svg>,
    bold: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z" /><path d="M6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z" /></svg>,
    italic: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="19" y1="4" x2="10" y2="4" /><line x1="14" y1="20" x2="5" y2="20" /><line x1="15" y1="4" x2="9" y2="20" /></svg>,
    strikethrough: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M16 4H9a3 3 0 0 0-2.83 4" /><path d="M14 12a4 4 0 0 1 0 8H6" /><line x1="4" y1="12" x2="20" y2="12" /></svg>,
    quote: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 21c3 0 7-1 7-8V5c0-1.25-.756-2.017-2-2H4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2 1 0 1 0 1 1v1c0 1-1 2-2 2s-1 .008-1 1.031V21z" /><path d="M15 21c3 0 7-1 7-8V5c0-1.25-.757-2.017-2-2h-4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2h.75c0 2.25.25 4-2.75 4v3z" /></svg>,
    ul: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="8" y1="6" x2="21" y2="6" /><line x1="8" y1="12" x2="21" y2="12" /><line x1="8" y1="18" x2="21" y2="18" /><line x1="3" y1="6" x2="3.01" y2="6" /><line x1="3" y1="12" x2="3.01" y2="12" /><line x1="3" y1="18" x2="3.01" y2="18" /></svg>,
    ol: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="10" y1="6" x2="21" y2="6" /><line x1="10" y1="12" x2="21" y2="12" /><line x1="10" y1="18" x2="21" y2="18" /><path d="M4 6h1v4" /><path d="M4 10h2" /><path d="M6 18H4c0-1 2-2 2-3s-1-1.5-2-1" /></svg>,
    check: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="9 11 12 14 22 4" /><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" /></svg>,
    code: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" /></svg>,
    inlineCode: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m18 16 4-4-4-4" /><path d="m6 8-4 4 4 4" /><path d="m14.5 4-5 16" /></svg>,
    link: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" /><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" /></svg>,
    table: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2" /><line x1="3" y1="9" x2="21" y2="9" /><line x1="3" y1="15" x2="21" y2="15" /><line x1="9" y1="3" x2="9" y2="21" /><line x1="15" y1="3" x2="15" y2="21" /></svg>,
    image: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2" /><circle cx="8.5" cy="8.5" r="1.5" /><polyline points="21 15 16 10 5 21" /></svg>,
    emoji: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10" /><path d="M8 14s1.5 2 4 2 4-2 4-2" /><line x1="9" y1="9" x2="9.01" y2="9" /><line x1="15" y1="9" x2="15.01" y2="9" /></svg>,
    undo: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="1 4 1 10 7 10" /><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" /></svg>,
    redo: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="23 4 23 10 17 10" /><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" /></svg>,
    preview: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>,
    fullscreen: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="15 3 21 3 21 9" /><polyline points="9 21 3 21 3 15" /><line x1="21" y1="3" x2="14" y2="10" /><line x1="3" y1="21" x2="10" y2="14" /></svg>,
    exitFullscreen: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="4 14 10 14 10 20" /><polyline points="20 10 14 10 14 4" /><line x1="14" y1="10" x2="21" y2="3" /><line x1="3" y1="21" x2="10" y2="14" /></svg>,
    plugin: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="16" /><line x1="8" y1="12" x2="16" y2="12" /></svg>,
    hr: <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="2" y1="12" x2="22" y2="12" /></svg>,
};

// ── Theme compartment for live dark/light switching ─────────
const themeCompartment = new Compartment();

function lightTheme() {
    return EditorView.theme({
        "&": { backgroundColor: "hsl(var(--card))", color: "hsl(var(--foreground))" },
        "&.cm-focused": { outline: "none" },
        ".cm-gutters": { backgroundColor: "hsl(var(--card))", borderRight: "1px solid hsl(var(--border))" },
        ".cm-activeLineGutter": { backgroundColor: "hsl(var(--accent))" },
        ".cm-activeLine": { backgroundColor: "hsl(var(--accent) / 0.3)", border: "none" },
        ".cm-cursor": { borderLeftColor: "hsl(var(--foreground))" },
        ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": { backgroundColor: "hsl(var(--primary) / 0.15) !important" },
        ".cm-content": { caretColor: "hsl(var(--foreground))" },
    });
}

// ── Helper: wrap selection or insert at cursor ──────────────
function wrapSelection(view: EditorView, before: string, after: string) {
    const { from, to } = view.state.selection.main;
    const selected = view.state.sliceDoc(from, to);
    view.dispatch({
        changes: { from, to, insert: before + selected + after },
        selection: { anchor: from + before.length, head: from + before.length + selected.length },
    });
    view.focus();
}

function insertAtLineStart(view: EditorView, prefix: string) {
    const { from } = view.state.selection.main;
    const line = view.state.doc.lineAt(from);
    // Toggle: if line already starts with prefix, remove it
    if (line.text.startsWith(prefix)) {
        view.dispatch({ changes: { from: line.from, to: line.from + prefix.length, insert: "" } });
    } else {
        view.dispatch({ changes: { from: line.from, insert: prefix } });
    }
    view.focus();
}

function insertText(view: EditorView, text: string) {
    const { from } = view.state.selection.main;
    view.dispatch({ changes: { from, insert: text }, selection: { anchor: from + text.length } });
    view.focus();
}

// ── Component ───────────────────────────────────────────────
const MarkdownEditor = forwardRef<MarkdownEditorRef, MarkdownEditorProps>(
    ({ initialValue = "", onChange, pluginButtons = [], placeholder, minHeight = 400 }, ref) => {
        const editorContainerRef = useRef<HTMLDivElement>(null);
        const viewRef = useRef<EditorView | null>(null);
        const onChangeRef = useRef(onChange);
        const initialValueRef = useRef(initialValue);
        const placeholderRef = useRef(placeholder || "");
        const showPreviewRef = useRef(false);
        const mobileTabRef = useRef<"edit" | "preview">("edit");
        const fetchPreviewRef = useRef<(content: string) => void>(() => { });
        const handleFileUploadRef = useRef<(file: File) => Promise<void>>(async () => { });
        const locale = useI18nStore((s) => s.locale);

        const [showPreview, setShowPreview] = useState(false);
        const [mobileTab, setMobileTab] = useState<"edit" | "preview">("edit");

        // Keep refs in sync for use inside the CodeMirror updateListener
        showPreviewRef.current = showPreview;
        mobileTabRef.current = mobileTab;
        const [previewHtml, setPreviewHtml] = useState("");
        const [previewLoading, setPreviewLoading] = useState(false);
        const [isFullscreen, setIsFullscreen] = useState(false);
        const [emojiPickerOpen, setEmojiPickerOpen] = useState(false);
        const [emojiPickerPos, setEmojiPickerPos] = useState({ top: 0, left: 0 });
        const emojiButtonRef = useRef<HTMLButtonElement>(null);
        const [pluginMenuOpen, setPluginMenuOpen] = useState(false);
        const pluginMenuRef = useRef<HTMLDivElement>(null);
        const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
        const previewAbortRef = useRef<AbortController | null>(null);
        const previewRequestIdRef = useRef(0);
        const wrapperRef = useRef<HTMLDivElement>(null);
        const [uploadPanelOpen, setUploadPanelOpen] = useState(false);
        const [isDragOver, setIsDragOver] = useState(false);
        const uploadPanelRef = useRef<HTMLDivElement>(null);
        const uploadButtonRef = useRef<HTMLButtonElement>(null);
        const fileInputRef = useRef<HTMLInputElement>(null);
        // File browser (library) state
        const [uploadTab, setUploadTab] = useState<"upload" | "library">("upload");
        const [libraryFiles, setLibraryFiles] = useState<FileInfo[]>([]);
        const [libraryLoading, setLibraryLoading] = useState(false);
        const [librarySearch, setLibrarySearch] = useState("");
        const libraryRequestIdRef = useRef(0);
        const librarySearchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
        // Image resize state
        const [pendingImage, setPendingImage] = useState<{ name: string; url: string } | null>(null);
        const [imageSize, setImageSize] = useState<string>("100%");

        onChangeRef.current = onChange;

        // ── Ref API ─────────────────────────────────────────────
        useImperativeHandle(ref, () => ({
            getValue: () => viewRef.current?.state.doc.toString() || "",
            setValue: (value: string) => {
                const view = viewRef.current;
                if (view) {
                    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
                }
            },
            insertValue: (value: string) => {
                const view = viewRef.current;
                if (view) {
                    insertText(view, value);
                }
            },
            focus: () => viewRef.current?.focus(),
        }));

        // ── Preview fetch (debounced) ───────────────────────────
        const fetchPreview = useCallback((content: string) => {
            const requestId = ++previewRequestIdRef.current;
            previewAbortRef.current?.abort();
            if (previewTimerRef.current) clearTimeout(previewTimerRef.current);
            previewTimerRef.current = setTimeout(async () => {
                if (!content.trim()) {
                    setPreviewHtml("");
                    setPreviewLoading(false);
                    return;
                }
                const controller = new AbortController();
                previewAbortRef.current = controller;
                setPreviewLoading(true);
                try {
                    const { data } = await api.post<{ html: string }>("/site/render", { content }, {
                        signal: controller.signal,
                    });
                    if (requestId !== previewRequestIdRef.current) return;
                    setPreviewHtml(data.html || "");
                } catch {
                    if (controller.signal.aborted || requestId !== previewRequestIdRef.current) return;
                    setPreviewHtml("<p style='color:red'>Preview failed</p>");
                } finally {
                    if (requestId === previewRequestIdRef.current) {
                        setPreviewLoading(false);
                        if (previewAbortRef.current === controller) {
                            previewAbortRef.current = null;
                        }
                    }
                }
            }, 400);
        }, []);

        useEffect(() => {
            return () => {
                previewAbortRef.current?.abort();
                if (previewTimerRef.current) {
                    clearTimeout(previewTimerRef.current);
                }
            };
        }, []);

        useEffect(() => {
            return () => {
                libraryRequestIdRef.current += 1;
                if (librarySearchTimerRef.current) {
                    clearTimeout(librarySearchTimerRef.current);
                }
            };
        }, []);

        // Keep fetchPreview ref in sync
        fetchPreviewRef.current = fetchPreview;

        // ── Init CodeMirror ─────────────────────────────────────
        useEffect(() => {
            if (!editorContainerRef.current) return;

            const isDark = document.documentElement.classList.contains("dark");

            const updateListener = EditorView.updateListener.of((update: ViewUpdate) => {
                if (update.docChanged) {
                    const value = update.state.doc.toString();
                    onChangeRef.current?.(value);
                    // Real-time preview: refresh when content changes while preview is visible
                    if (showPreviewRef.current || mobileTabRef.current === "preview") {
                        fetchPreviewRef.current(value);
                    }
                }
            });

            const state = EditorState.create({
                doc: initialValueRef.current,
                extensions: [
                    history(),
                    keymap.of([...defaultKeymap, ...historyKeymap]),
                    markdown({ base: markdownLanguage }),
                    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
                    themeCompartment.of(isDark ? oneDark : lightTheme()),
                    EditorView.lineWrapping,
                    cmPlaceholder(placeholderRef.current),
                    updateListener,
                    EditorView.theme({
                        "&": { height: "100%" },
                        ".cm-content": { minHeight: "100%" },
                        ".cm-scroller": { overflow: "auto" },
                    }),
                ],
            });

            const view = new EditorView({
                state,
                parent: editorContainerRef.current,
            });

            viewRef.current = view;

            // ── Upload: paste & drop handler ──────────────────────
            const handlePaste = async (e: ClipboardEvent) => {
                const items = e.clipboardData?.items;
                if (!items) return;
                for (const item of Array.from(items)) {
                    if (item.kind === "file") {
                        e.preventDefault();
                        const file = item.getAsFile();
                        if (file) await handleFileUploadRef.current(file);
                        break;
                    }
                }
            };

            const handleDrop = async (e: DragEvent) => {
                const files = e.dataTransfer?.files;
                if (!files || files.length === 0) return;
                e.preventDefault();
                for (const file of Array.from(files)) {
                    await handleFileUploadRef.current(file);
                }
            };

            const dom = view.dom;
            dom.addEventListener("paste", handlePaste);
            dom.addEventListener("drop", handleDrop);

            return () => {
                dom.removeEventListener("paste", handlePaste);
                dom.removeEventListener("drop", handleDrop);
                view.destroy();
                viewRef.current = null;
            };
        }, []);

        // ── Dark mode sync ──────────────────────────────────────
        useEffect(() => {
            const observer = new MutationObserver(() => {
                const isDark = document.documentElement.classList.contains("dark");
                viewRef.current?.dispatch({
                    effects: themeCompartment.reconfigure(isDark ? oneDark : lightTheme()),
                });
            });
            observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
            return () => observer.disconnect();
        }, []);

        // ── Update preview when content changes or preview toggled ─
        useEffect(() => {
            if (showPreview || mobileTab === "preview") {
                const content = viewRef.current?.state.doc.toString() || "";
                fetchPreview(content);
            }
            return () => { if (previewTimerRef.current) clearTimeout(previewTimerRef.current); };
        }, [showPreview, mobileTab, fetchPreview]);

        // ── Close upload panel on outside click ─────────────────
        useEffect(() => {
            if (!uploadPanelOpen) return;
            const handleClick = (e: MouseEvent) => {
                if (uploadPanelRef.current && !uploadPanelRef.current.contains(e.target as Node) &&
                    uploadButtonRef.current && !uploadButtonRef.current.contains(e.target as Node)) {
                    setUploadPanelOpen(false);
                }
            };
            document.addEventListener("mousedown", handleClick);
            return () => document.removeEventListener("mousedown", handleClick);
        }, [uploadPanelOpen]);

        // ── Close plugin menu on outside click ──────────────────
        useEffect(() => {
            if (!pluginMenuOpen) return;
            const handleClick = (e: MouseEvent) => {
                if (pluginMenuRef.current && !pluginMenuRef.current.contains(e.target as Node)) {
                    setPluginMenuOpen(false);
                }
            };
            document.addEventListener("mousedown", handleClick);
            return () => document.removeEventListener("mousedown", handleClick);
        }, [pluginMenuOpen]);

        // ── Fullscreen escape ───────────────────────────────────
        useEffect(() => {
            if (!isFullscreen) return;
            const handleEsc = (e: KeyboardEvent) => { if (e.key === "Escape") setIsFullscreen(false); };
            document.addEventListener("keydown", handleEsc);
            return () => document.removeEventListener("keydown", handleEsc);
        }, [isFullscreen]);

        // ── Upload handler ──────────────────────────────────────
        const [uploading, setUploading] = useState(false);
        const MAX_UPLOAD_SIZE = 10 * 1024 * 1024; // 10MB
        const BLOCKED_EXTENSIONS = [".exe", ".bat", ".sh", ".cmd", ".msi", ".dll", ".com", ".scr"];

        const handleFileUpload = async (file: File) => {
            const view = viewRef.current;
            if (!view) return;

            // Validate file size
            if (file.size > MAX_UPLOAD_SIZE) {
                const maxStr = `${MAX_UPLOAD_SIZE / 1024 / 1024}MB`;
                const sizeStr = file.size < 1024 * 1024
                    ? `${(file.size / 1024).toFixed(1)}KB`
                    : `${(file.size / 1024 / 1024).toFixed(1)}MB`;
                toast.error(i18nT("editor.fileTooLarge").replace("{max}", maxStr).replace("{size}", sizeStr) || `File too large: ${sizeStr} (max ${maxStr})`);
                return;
            }

            // Validate file type
            const ext = ("." + file.name.split(".").pop()?.toLowerCase()) || "";
            if (BLOCKED_EXTENSIONS.includes(ext)) {
                toast.error(i18nT("editor.fileTypeBlocked").replace("{ext}", ext) || `File type ${ext} is not allowed`);
                return;
            }

            setUploading(true);
            try {
                const isImage = file.type.startsWith("image/");
                const { data } = isImage ? await uploadApi.image(file) : await uploadApi.file(file);
                if (isImage) {
                    setPendingImage({ name: file.name, url: data.url });
                    setImageSize("100%");
                } else {
                    const sizeStr = data.size < 1024 * 1024
                        ? `${(data.size / 1024).toFixed(1)} KB`
                        : `${(data.size / 1024 / 1024).toFixed(1)} MB`;
                    insertText(view, `[file name="${file.name}" size="${sizeStr}" url="${data.url}" /]`);
                    setUploadPanelOpen(false);
                }
            } catch (error) {
                console.error("Upload failed:", error);
            } finally {
                setUploading(false);
            }
        };
        handleFileUploadRef.current = handleFileUpload;

        // Insert image with optional resize
        const insertImageWithSize = (name: string, url: string, size: string) => {
            const view = viewRef.current;
            if (!view) return;
            if (size && size !== "100%") {
                insertText(view, `![${name}|${size}](${url})`);
            } else {
                insertText(view, `![${name}](${url})`);
            }
            setPendingImage(null);
            setUploadPanelOpen(false);
        };

        // Load library files
        const loadLibraryFiles = async (search?: string) => {
            const requestId = ++libraryRequestIdRef.current;
            setLibraryLoading(true);
            try {
                const res = await filesApi.list({ search: search || undefined });
                if (requestId === libraryRequestIdRef.current) {
                    setLibraryFiles(res.data.files);
                }
            } catch {
                if (requestId === libraryRequestIdRef.current) {
                    setLibraryFiles([]);
                }
            } finally {
                if (requestId === libraryRequestIdRef.current) {
                    setLibraryLoading(false);
                }
            }
        };

        const scheduleLibrarySearch = (search: string) => {
            if (librarySearchTimerRef.current) {
                clearTimeout(librarySearchTimerRef.current);
            }
            librarySearchTimerRef.current = setTimeout(() => {
                void loadLibraryFiles(search.trim());
            }, 250);
        };

        // Insert file from library
        const insertLibraryFile = (file: FileInfo) => {
            const view = viewRef.current;
            if (!view) return;
            if (file.is_image) {
                setPendingImage({ name: file.name, url: file.url });
                setImageSize("100%");
            } else {
                const sizeStr = file.size < 1024 * 1024
                    ? `${(file.size / 1024).toFixed(1)} KB`
                    : `${(file.size / 1024 / 1024).toFixed(1)} MB`;
                insertText(view, `[file name="${file.name}" size="${sizeStr}" url="${file.url}" /]`);
                setUploadPanelOpen(false);
            }
        };

        // ── Toolbar handlers ────────────────────────────────────
        const tb = useMemo(() => {
            const v = () => viewRef.current;
            return {
                heading: () => v() && insertAtLineStart(v()!, "## "),
                bold: () => v() && wrapSelection(v()!, "**", "**"),
                italic: () => v() && wrapSelection(v()!, "*", "*"),
                strike: () => v() && wrapSelection(v()!, "~~", "~~"),
                quote: () => v() && insertAtLineStart(v()!, "> "),
                ul: () => v() && insertAtLineStart(v()!, "- "),
                ol: () => v() && insertAtLineStart(v()!, "1. "),
                check: () => v() && insertAtLineStart(v()!, "- [ ] "),
                code: () => v() && wrapSelection(v()!, "\n```\n", "\n```\n"),
                inlineCode: () => v() && wrapSelection(v()!, "`", "`"),
                link: () => v() && wrapSelection(v()!, "[", "](url)"),
                table: () => v() && insertText(v()!, "\n| Header | Header |\n| ------ | ------ |\n| Cell   | Cell   |\n"),
                hr: () => v() && insertText(v()!, "\n---\n"),
                upload: () => { },  // handled by panel toggle
                undo: () => v() && undo(v()!),
                redo: () => v() && redo(v()!),
            };
        }, []);

        const handleEmojiSelect = useCallback((emoji: string) => {
            const view = viewRef.current;
            if (view) insertText(view, emoji);
            setEmojiPickerOpen(false);
        }, []);

        const handleEmojiButtonClick = useCallback(() => {
            if (emojiButtonRef.current) {
                const rect = emojiButtonRef.current.getBoundingClientRect();
                setEmojiPickerPos({ top: rect.bottom + 4, left: Math.max(8, rect.left - 150) });
            }
            setEmojiPickerOpen((prev) => !prev);
        }, []);

        const togglePreview = useCallback(() => {
            setShowPreview((prev) => {
                if (!prev) {
                    const content = viewRef.current?.state.doc.toString() || "";
                    fetchPreview(content);
                }
                return !prev;
            });
        }, [fetchPreview]);

        const handleMobileTabChange = useCallback((tab: string) => {
            setMobileTab(tab as "edit" | "preview");
            if (tab === "preview") {
                const content = viewRef.current?.state.doc.toString() || "";
                fetchPreview(content);
            }
        }, [fetchPreview]);

        // ── Render toolbar ──────────────────────────────────────
        const ToolbarBtn = ({ icon, title, onClick, active }: { icon: React.ReactNode; title: string; onClick: () => void; active?: boolean }) => (
            <Button
                type="button"
                variant="ghost"
                size="icon"
                className={`h-8 w-8 ${active ? "bg-primary/10 text-primary" : ""}`}
                title={title}
                onClick={onClick}
            >
                {icon}
            </Button>
        );

        const Sep = () => <div className="w-px h-5 bg-border mx-0.5" />;

        // ── Preview panel ───────────────────────────────────────
        const PreviewPanel = () => (
            <div
                className="prose prose-sm dark:prose-invert h-full max-w-none overflow-auto p-4 [&_img]:max-w-full [&_img]:h-auto [&_img.emoji]:!w-[1.2em] [&_img.emoji]:!h-[1.2em] [&_img.emoji]:!inline-block [&_img.emoji]:!m-0 [&_img.emoji]:!align-[-0.1em] [&_img.twemoji]:!w-[1.2em] [&_img.twemoji]:!h-[1.2em] [&_img.twemoji]:!inline-block [&_img.twemoji]:!m-0 [&_img.twemoji]:!align-[-0.1em]"
            >
                {previewLoading ? (
                    <div className="flex items-center justify-center py-8">
                        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                    </div>
                ) : previewHtml ? (
                    <div
                        dangerouslySetInnerHTML={{ __html: previewHtml }}
                    />
                ) : (
                    <p className="text-muted-foreground">{i18nT("editor.previewEmpty")}</p>
                )}
            </div>
        );

        // Hidden file input for upload
        const FileInput = () => (
            <input
                ref={fileInputRef}
                type="file"
                className="hidden"
                accept="*/*"
                onChange={async (e) => {
                    const file = e.target.files?.[0];
                    if (file) await handleFileUpload(file);
                    e.target.value = "";
                }}
            />
        );

        const editorAreaStyle = isFullscreen ? undefined : { height: `${minHeight}px` };
        const editorPaneStyle = { height: "100%" };

        return (
            <div
                ref={wrapperRef}
                className={`border rounded-md overflow-hidden bg-card ${isFullscreen ? "fixed inset-0 z-50 rounded-none border-none flex flex-col" : ""
                    }`}
            >
                {/* Toolbar */}
                <div className="flex items-center flex-wrap gap-0.5 px-2 py-1 border-b bg-card">
                    <ToolbarBtn icon={icons.heading} title={i18nT("editor.heading")} onClick={tb.heading} />
                    <ToolbarBtn icon={icons.bold} title={i18nT("editor.bold")} onClick={tb.bold} />
                    <ToolbarBtn icon={icons.italic} title={i18nT("editor.italic")} onClick={tb.italic} />
                    <ToolbarBtn icon={icons.strikethrough} title={i18nT("editor.strikethrough")} onClick={tb.strike} />
                    <Sep />
                    <ToolbarBtn icon={icons.hr} title={i18nT("editor.hr")} onClick={tb.hr} />
                    <ToolbarBtn icon={icons.quote} title={i18nT("editor.quote")} onClick={tb.quote} />
                    <ToolbarBtn icon={icons.ul} title={i18nT("editor.ul")} onClick={tb.ul} />
                    <ToolbarBtn icon={icons.ol} title={i18nT("editor.ol")} onClick={tb.ol} />
                    <ToolbarBtn icon={icons.check} title={i18nT("editor.check")} onClick={tb.check} />
                    <Sep />
                    <ToolbarBtn icon={icons.code} title={i18nT("editor.code")} onClick={tb.code} />
                    <ToolbarBtn icon={icons.inlineCode} title={i18nT("editor.inlineCode")} onClick={tb.inlineCode} />
                    <ToolbarBtn icon={icons.link} title={i18nT("editor.link")} onClick={tb.link} />
                    <ToolbarBtn icon={icons.table} title={i18nT("editor.table")} onClick={tb.table} />
                    <Sep />
                    <div className="relative">
                        <Button
                            ref={uploadButtonRef}
                            type="button"
                            variant="ghost"
                            size="icon"
                            className={`h-8 w-8 ${uploadPanelOpen ? "bg-primary/10 text-primary" : ""}`}
                            title={i18nT("editor.upload")}
                            onClick={() => setUploadPanelOpen(!uploadPanelOpen)}
                        >
                            {icons.image}
                        </Button>
                        {uploadPanelOpen && (
                            <div
                                ref={uploadPanelRef}
                                className="absolute top-full left-1/2 -translate-x-1/2 mt-2 z-50 w-80 bg-popover border rounded-lg shadow-lg"
                            >
                                {/* Image resize picker */}
                                {pendingImage ? (
                                    <div className="p-4">
                                        <div className="mb-3">
                                            <img src={pendingImage.url} alt={pendingImage.name} className="w-full h-32 object-contain rounded border bg-muted" />
                                        </div>
                                        <p className="text-xs text-muted-foreground mb-2 truncate">{pendingImage.name}</p>
                                        <p className="text-xs font-medium mb-2">{i18nT("editor.imageSize")}</p>
                                        <div className="flex gap-2 mb-3">
                                            {["100%", "75%", "50%", "25%"].map((s) => (
                                                <button
                                                    key={s}
                                                    className={`flex-1 px-2 py-1.5 text-xs rounded border transition-colors ${imageSize === s ? "bg-primary text-primary-foreground border-primary" : "hover:bg-accent"}`}
                                                    onClick={() => setImageSize(s)}
                                                >
                                                    {s}
                                                </button>
                                            ))}
                                        </div>
                                        <div className="flex gap-2">
                                            <Button size="sm" className="flex-1" onClick={() => insertImageWithSize(pendingImage.name, pendingImage.url, imageSize)}>
                                                {i18nT("editor.insertImage")}
                                            </Button>
                                            <Button size="sm" variant="ghost" onClick={() => setPendingImage(null)}>
                                                {i18nT("common.cancel")}
                                            </Button>
                                        </div>
                                    </div>
                                ) : (
                                    <>
                                        {/* Tab headers */}
                                        <div className="flex border-b">
                                            <button
                                                className={`flex-1 px-3 py-2 text-xs font-medium transition-colors ${uploadTab === "upload" ? "border-b-2 border-primary text-primary" : "text-muted-foreground hover:text-foreground"}`}
                                                onClick={() => setUploadTab("upload")}
                                            >
                                                {i18nT("editor.uploadTab")}
                                            </button>
                                            <button
                                                className={`flex-1 px-3 py-2 text-xs font-medium transition-colors ${uploadTab === "library" ? "border-b-2 border-primary text-primary" : "text-muted-foreground hover:text-foreground"}`}
                                                onClick={() => {
                                                    setUploadTab("library");
                                                    if (libraryFiles.length === 0) loadLibraryFiles();
                                                }}
                                            >
                                                {i18nT("editor.libraryTab")}
                                            </button>
                                        </div>
                                        <div className="p-4">
                                            {uploadTab === "upload" ? (
                                                /* Dropzone */
                                                <div
                                                    className={`relative flex flex-col items-center justify-center gap-3 p-6 border-2 border-dashed rounded-lg cursor-pointer transition-colors ${isDragOver
                                                        ? "border-primary bg-primary/5"
                                                        : "border-muted-foreground/30 hover:border-primary/50 hover:bg-accent/50"
                                                        }`}
                                                    onClick={() => fileInputRef.current?.click()}
                                                    onDragOver={(e) => { e.preventDefault(); e.stopPropagation(); setIsDragOver(true); }}
                                                    onDragLeave={(e) => { e.preventDefault(); e.stopPropagation(); setIsDragOver(false); }}
                                                    onDrop={async (e) => {
                                                        e.preventDefault(); e.stopPropagation(); setIsDragOver(false);
                                                        const files = e.dataTransfer?.files;
                                                        if (files) for (const file of Array.from(files)) await handleFileUpload(file);
                                                    }}
                                                >
                                                    {uploading ? (
                                                        <Loader2 className="h-8 w-8 animate-spin text-primary" />
                                                    ) : (
                                                        <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-muted-foreground">
                                                            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                                                            <polyline points="17 8 12 3 7 8" />
                                                            <line x1="12" y1="3" x2="12" y2="15" />
                                                        </svg>
                                                    )}
                                                    <div className="text-center">
                                                        <p className="text-sm font-medium">{i18nT("editor.dropzoneTitle")}</p>
                                                        <p className="text-xs text-muted-foreground mt-1">{i18nT("editor.dropzoneHint")}</p>
                                                    </div>
                                                </div>
                                            ) : (
                                                /* Library browser */
                                                <div>
                                                    <input
                                                        type="text"
                                                        placeholder={i18nT("editor.searchFiles")}
                                                        className="w-full px-2 py-1.5 text-sm border rounded mb-2 bg-background"
                                                        value={librarySearch}
                                                        onChange={(e) => {
                                                            setLibrarySearch(e.target.value);
                                                            scheduleLibrarySearch(e.target.value);
                                                        }}
                                                    />
                                                    <div className="max-h-48 overflow-y-auto space-y-1">
                                                        {libraryLoading ? (
                                                            <div className="flex justify-center py-4">
                                                                <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                                                            </div>
                                                        ) : libraryFiles.length === 0 ? (
                                                            <p className="text-xs text-muted-foreground text-center py-4">{i18nT("common.noData")}</p>
                                                        ) : (
                                                            libraryFiles.map((file) => (
                                                                <button
                                                                    key={file.name}
                                                                    className="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-accent text-left transition-colors"
                                                                    onClick={() => insertLibraryFile(file)}
                                                                >
                                                                    {file.is_image ? (
                                                                        <img src={file.url} alt={file.name} className="w-8 h-8 object-cover rounded border shrink-0" />
                                                                    ) : (
                                                                        <div className="w-8 h-8 rounded border bg-muted flex items-center justify-center shrink-0">
                                                                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" /></svg>
                                                                        </div>
                                                                    )}
                                                                    <div className="min-w-0 flex-1">
                                                                        <p className="text-xs truncate">{file.name}</p>
                                                                        <p className="text-[10px] text-muted-foreground">
                                                                            {file.size < 1024 * 1024 ? `${(file.size / 1024).toFixed(0)} KB` : `${(file.size / 1024 / 1024).toFixed(1)} MB`}
                                                                        </p>
                                                                    </div>
                                                                </button>
                                                            ))
                                                        )}
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    </>
                                )}
                            </div>
                        )}
                    </div>
                    <ToolbarBtn
                        icon={icons.emoji}
                        title={i18nT("editor.emoji")}
                        onClick={handleEmojiButtonClick}
                    />
                    <button ref={emojiButtonRef} className="hidden" />
                    {/* Plugin buttons dropdown */}
                    {pluginButtons.length > 0 && (
                        <>
                            <Sep />
                            <div className="relative" ref={pluginMenuRef}>
                                <ToolbarBtn icon={icons.plugin} title={i18nT("editor.plugins")} onClick={() => setPluginMenuOpen(!pluginMenuOpen)} />
                                {pluginMenuOpen && (
                                    <div className="absolute top-full left-0 mt-1 z-50 bg-popover border rounded-md shadow-md py-1 min-w-[160px]">
                                        {pluginButtons.map((btn) => {
                                            const label = resolveLabel(btn.label, locale);
                                            return (
                                                <button
                                                    key={btn.id}
                                                    className="w-full text-left px-3 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground transition-colors"
                                                    onClick={() => {
                                                        const view = viewRef.current;
                                                        if (view) insertText(view, btn.insertBefore + btn.insertAfter);
                                                        setPluginMenuOpen(false);
                                                    }}
                                                >
                                                    {label}
                                                </button>
                                            );
                                        })}
                                    </div>
                                )}
                            </div>
                        </>
                    )}
                    <Sep />
                    <ToolbarBtn icon={icons.undo} title="Undo" onClick={tb.undo} />
                    <ToolbarBtn icon={icons.redo} title="Redo" onClick={tb.redo} />
                    <Sep />
                    {/* Desktop: preview toggle */}
                    <div className="hidden md:flex">
                        <ToolbarBtn icon={icons.preview} title={i18nT("editor.preview")} onClick={togglePreview} active={showPreview} />
                    </div>
                    <ToolbarBtn
                        icon={isFullscreen ? icons.exitFullscreen : icons.fullscreen}
                        title={i18nT("editor.fullscreen")}
                        onClick={() => setIsFullscreen(!isFullscreen)}
                    />
                </div>

                {/* Mobile: tab switcher */}
                <div className="md:hidden border-b">
                    <Tabs value={mobileTab} onValueChange={handleMobileTabChange}>
                        <TabsList className="w-full rounded-none h-9">
                            <TabsTrigger value="edit" className="flex-1 text-xs">{i18nT("editor.edit")}</TabsTrigger>
                            <TabsTrigger value="preview" className="flex-1 text-xs">{i18nT("editor.preview")}</TabsTrigger>
                        </TabsList>
                    </Tabs>
                </div>

                {/* Editor + Preview area */}
                <div
                    className={`min-h-0 overflow-hidden ${isFullscreen ? "flex-1" : ""} ${showPreview ? "md:grid md:grid-cols-2 md:divide-x" : ""}`}
                    style={editorAreaStyle}
                >
                    {/* Editor (hidden on mobile when preview tab is active) */}
                    <div
                        ref={editorContainerRef}
                        className={`${mobileTab === "preview" ? "hidden md:block" : ""} min-h-0 overflow-hidden`}
                        style={editorPaneStyle}
                    />
                    {/* Desktop preview panel */}
                    {showPreview && (
                        <div className="hidden min-h-0 overflow-hidden md:block" style={editorPaneStyle}>
                            <PreviewPanel />
                        </div>
                    )}
                    {/* Mobile preview */}
                    {mobileTab === "preview" && (
                        <div className="min-h-0 overflow-hidden md:hidden" style={editorPaneStyle}>
                            <PreviewPanel />
                        </div>
                    )}
                </div>

                <FileInput />

                {/* Emoji picker portal */}
                {emojiPickerOpen &&
                    createPortal(
                        <div style={{ position: "fixed", top: 0, left: 0, right: 0, bottom: 0, zIndex: 60 }} onClick={() => setEmojiPickerOpen(false)}>
                            <div
                                style={{ position: "fixed", top: emojiPickerPos.top, left: emojiPickerPos.left, zIndex: 61 }}
                                onClick={(e) => e.stopPropagation()}
                            >
                                <Suspense
                                    fallback={
                                        <div className="flex h-[300px] w-[360px] items-center justify-center rounded-lg border bg-popover shadow-xl">
                                            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                                        </div>
                                    }
                                >
                                    <EmojiPicker onSelect={handleEmojiSelect} onClose={() => setEmojiPickerOpen(false)} />
                                </Suspense>
                            </div>
                        </div>,
                        document.body
                    )}
            </div>
        );
    }
);

MarkdownEditor.displayName = "MarkdownEditor";
export default MarkdownEditor;
export type { MarkdownEditorRef as VditorEditorRef }; // backward compat alias
