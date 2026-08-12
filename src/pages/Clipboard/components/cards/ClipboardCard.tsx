import { useUnmount } from "ahooks";
import type {
  DragEvent,
  FC,
  KeyboardEvent,
  MouseEvent,
  PointerEvent,
  Ref,
} from "react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  type ContextMenuPayload,
  popupClipboardItemMenu,
  setClipboardWindowAutoHideSuspended,
  startDragClipboardItem,
} from "@/commands";
import AssetImage from "@/components/AssetImage";
import KeyHint from "@/components/KeyHint";
import type { ItemActionLabels } from "@/constants/itemActions";
import type { ClipboardAction, ClipboardItem } from "@/types/clipboard";
import type { ItemAction } from "@/types/settings";
import { cn } from "@/utils/cn";
import { isMac, isWin } from "@/utils/is";
import ClipboardQuickActions from "./ClipboardQuickActions";
import FilesCard from "./FilesCard";
import ImageCard from "./ImageCard";
import NoteContentSwitcher from "./NoteContentSwitcher";
import TextCard from "./TextCard";

interface ClipboardCardProps {
  item: ClipboardItem;
  isSelected?: boolean;
  /**
   * 列表滚动期间冻结卡片 hover，避免快捷操作和备注内容切换触发重排。
   */
  isListScrolling?: boolean;
  /**
   * 快捷键提示字符（"1"–"9" / "0"），存在时在 app 图标上叠加 KeyHint；
   * 按下修饰键（macOS ⌘ / Windows Ctrl）+ 该数字键触发快速粘贴。
   */
  hintKey?: string;
  /**
   * 快捷键触发时执行的粘贴操作，由父级列表注入。
   */
  onQuickPaste?: () => void;
  /**
   * MOD 键按下时，URL / Email 文本以链接态展示。
   */
  isLinkActive?: boolean;
  /**
   * 点击 URL / Email 文本时打开外部链接。
   */
  onOpenLink?: () => void;
  onPointerEnter?: (event: PointerEvent<HTMLDivElement>) => void;
  onPointerLeave?: () => void;
  onPointerMove?: (event: PointerEvent<HTMLDivElement>) => void;
  onActivate?: () => void;
  onMouseDown?: (event: MouseEvent<HTMLDivElement>) => void;
  onAuxClick?: (event: MouseEvent<HTMLDivElement>) => void;
  onDoubleClick?: (event: MouseEvent<HTMLDivElement>) => void;
  onContextMenuOpen?: (
    payload: ContextMenuPayload,
    x: number,
    y: number,
  ) => void;
  availableActions?: ClipboardAction[];
  quickActions?: ItemAction[];
  quickActionLabels?: ItemActionLabels;
  onQuickAction?: (action: ItemAction) => Promise<void> | void;
  showOriginalOnHover?: boolean;
  rootRef?: Ref<HTMLDivElement>;
}

/**
 * 按 `kind` 分发到具体卡片组件，统一外层 padding / 时间戳 / 来源应用图标。
 * `isSelected` 为 true 时高亮背景与边框；指针事件由列表注入用于 hover preview；
 * 右键根节点弹出 Rust 端原生菜单（避免 tauri-apps/tauri#9470 的 muda use-after-free），
 * 点击菜单项后由列表层订阅 `clipboard://menu-action` 派发到实际处理逻辑。
 */
const ClipboardCard: FC<ClipboardCardProps> = (props) => {
  const {
    item,
    isSelected,
    isListScrolling = false,
    hintKey,
    onQuickPaste,
    isLinkActive,
    onOpenLink,
    onPointerEnter,
    onPointerLeave,
    onPointerMove,
    onActivate,
    onMouseDown,
    onAuxClick,
    onDoubleClick,
    onContextMenuOpen,
    availableActions,
    quickActions = [],
    quickActionLabels,
    onQuickAction,
    showOriginalOnHover = true,
    rootRef,
  } = props;
  const { kind, sourceAppId, subKind, sourceAppIconPath, sourceAppName } = item;
  const { t } = useTranslation("clipboard");
  const [hovered, setHovered] = useState(false);
  const pointerDownPositionRef = useRef<{ x: number; y: number } | null>(null);
  const autoHideSuspendedRef = useRef(false);
  const typeKey = subKind ?? kind;
  const typeLabel = t(`types.${typeKey}`);
  const body = renderBody(item, isLinkActive, onOpenLink);
  const showSensitiveIndicator = item.isSensitive && item.kind === "text";
  const showStatusIndicators = item.isPinned || showSensitiveIndicator;
  const sourceAppIcon = sourceAppId ? (
    <AssetImage
      alt={sourceAppName}
      className="size-4"
      src={sourceAppIconPath}
    />
  ) : (
    <img
      alt="EcoPaste"
      className="pointer-events-none size-4"
      src={isMac ? "/logo-mac.png" : "/logo.png"}
    />
  );

  const handleDragStart = async (event: DragEvent) => {
    event.preventDefault();

    if (!isWin) setAutoHideSuspended(true);

    await startDragClipboardItem(item.id);
  };

  const handleDragEnd = () => {
    pointerDownPositionRef.current = null;
    if (!isWin) setAutoHideSuspended(false);
  };

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;

    if (
      event.target instanceof Element &&
      event.target.closest(
        "button, input, textarea, select, a, [contenteditable='true']",
      )
    ) {
      return;
    }

    pointerDownPositionRef.current = { x: event.clientX, y: event.clientY };
  };

  const handlePointerUp = () => {
    pointerDownPositionRef.current = null;
    if (!isWin) setAutoHideSuspended(false);
  };

  const handlePointerCancel = () => {
    pointerDownPositionRef.current = null;
    if (!isWin) setAutoHideSuspended(false);
  };

  const handleClick = (event: MouseEvent<HTMLDivElement>) => {
    if (
      event.target instanceof Element &&
      event.target.closest(
        "button, input, textarea, select, a, [contenteditable='true']",
      )
    ) {
      return;
    }

    onActivate?.();
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Enter" && event.key !== " ") return;

    onActivate?.();
  };

  const handleCardPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const start = pointerDownPositionRef.current;
    if (isWin) {
      onPointerMove?.(event);
      return;
    }

    if (start && !autoHideSuspendedRef.current) {
      const movedX = event.clientX - start.x;
      const movedY = event.clientY - start.y;

      if (movedX * movedX + movedY * movedY >= 36) {
        setAutoHideSuspended(true);
      }
    }

    onPointerMove?.(event);
  };

  const setAutoHideSuspended = (suspended: boolean) => {
    if (autoHideSuspendedRef.current === suspended) return;

    autoHideSuspendedRef.current = suspended;
    void setClipboardWindowAutoHideSuspended(suspended);
  };

  useUnmount(() => {
    if (isWin || !autoHideSuspendedRef.current) return;

    void setClipboardWindowAutoHideSuspended(false);
  });

  const handleContextMenu = async (event: MouseEvent) => {
    event.preventDefault();
    const { clientX, clientY } = event;

    const actions = availableActions ?? item.availableActions ?? [];
    const { isFavorite, isPinned, note } = item;

    if (actions.length === 0) return;

    const payload = await popupClipboardItemMenu(
      item.id,
      [...actions],
      item.groupId,
      isFavorite,
      isPinned,
      Boolean(note),
    );
    if (!payload) return;

    onContextMenuOpen?.(payload, clientX, clientY);
  };

  const handlePointerEnter = (event: PointerEvent<HTMLDivElement>) => {
    if (isListScrolling) return;

    setHovered(true);
    onPointerEnter?.(event);
  };

  const handlePointerLeave = () => {
    if (isListScrolling) return;

    setHovered(false);
    onPointerLeave?.();
  };

  return (
    <div
      aria-selected={isSelected}
      className={cn(
        "relative flex flex-col gap-1 overflow-hidden rounded-2 border border-ant-border-secondary p-2 transition-colors duration-150 ease-out motion-reduce:transition-none",
        {
          "border-ant-primary bg-ant-blue-1": isSelected,
          "border-ant-primary bg-ant-container": item.isPinned && !isSelected,
        },
      )}
      draggable
      onAuxClick={onAuxClick}
      onClick={handleClick}
      onContextMenu={handleContextMenu}
      onDoubleClick={onDoubleClick}
      onDragEnd={handleDragEnd}
      onDragStart={handleDragStart}
      onKeyDown={handleKeyDown}
      onMouseDown={onMouseDown}
      onPointerCancel={handlePointerCancel}
      onPointerDown={handlePointerDown}
      onPointerEnter={handlePointerEnter}
      onPointerLeave={handlePointerLeave}
      onPointerMove={handleCardPointerMove}
      onPointerUp={handlePointerUp}
      ref={rootRef}
      role="option"
      tabIndex={-1}
    >
      <div className="flex items-center justify-between text-ant-secondary text-xs">
        <div className="flex min-w-0 items-center gap-1 overflow-hidden">
          {hintKey ? (
            <KeyHint hintKey={hintKey} onKeyPress={onQuickPaste}>
              {sourceAppIcon}
            </KeyHint>
          ) : (
            sourceAppIcon
          )}

          <span className="truncate">{typeLabel}</span>
        </div>

        <ClipboardQuickActions
          item={item}
          labels={quickActionLabels}
          onQuickAction={onQuickAction}
          quickActions={quickActions}
          visible={hovered}
        />
      </div>

      {item.note ? (
        <NoteContentSwitcher
          note={item.note}
          showOriginal={showOriginalOnHover && hovered}
        >
          {body}
        </NoteContentSwitcher>
      ) : (
        body
      )}
      {showStatusIndicators
        ? renderStatusIndicators(item.isPinned, showSensitiveIndicator)
        : null}
    </div>
  );
};

/**
 * 渲染卡片右下角的状态水印；仅表达状态，不参与交互。
 */
function renderStatusIndicators(isPinned: boolean, isSensitive: boolean) {
  return (
    <div className="pointer-events-none absolute right-2 bottom-2 flex items-end gap-1 text-ant-quaternary">
      {isPinned ? (
        <i aria-hidden="true" className="i-ph:push-pin-bold size-5" />
      ) : null}
      {isSensitive ? (
        <i aria-hidden="true" className="i-lucide:key-round size-5" />
      ) : null}
    </div>
  );
}

const renderBody = (
  item: ClipboardItem,
  isLinkActive?: boolean,
  onOpenLink?: () => void,
) => {
  if (item.kind === "image") return <ImageCard {...item} />;

  if (item.kind === "files") return <FilesCard {...item} />;

  return (
    <TextCard {...item} isLinkActive={isLinkActive} onOpenLink={onOpenLink} />
  );
};

export default ClipboardCard;
