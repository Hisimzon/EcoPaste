import {
  type FC,
  Fragment,
  type MouseEvent,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type {
  ContextMenuItemPayload,
  ContextMenuPayload,
} from "@/commands";
import type { ClipboardAction } from "@/types/clipboard";
import { cn } from "@/utils/cn";
import { formatShortcutDisplay } from "@/utils/shortcut";

interface ClipboardContextMenuProps {
  payload: ContextMenuPayload | null;
  x: number;
  y: number;
  onClose: () => void;
  onPick: (action: ClipboardAction, groupId?: string) => void;
}

interface MenuPosition {
  left: number;
  top: number;
}

const VIEWPORT_MARGIN = 8;

/**
 * 在剪贴板主 WebView 内渲染 Windows 右键菜单，避免创建额外 WebView2 窗口。
 * 分组选择使用同一面板切换，保证 360px 窄窗口内不会横向溢出。
 */
const ClipboardContextMenu: FC<ClipboardContextMenuProps> = (props) => {
  const { payload, x, y, onClose, onPick } = props;
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeSubmenu, setActiveSubmenu] =
    useState<ContextMenuItemPayload | null>(null);
  const [position, setPosition] = useState<MenuPosition>({ left: x, top: y });

  useEffect(() => {
    setActiveSubmenu(null);
  }, [payload]);

  useEffect(() => {
    if (!payload) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (menuRef.current?.contains(event.target as Node)) return;

      onClose();
    };

    window.addEventListener("pointerdown", handlePointerDown, true);

    return () => {
      window.removeEventListener("pointerdown", handlePointerDown, true);
    };
  }, [onClose, payload]);

  useLayoutEffect(() => {
    if (!payload || !menuRef.current) return;

    const bounds = menuRef.current.getBoundingClientRect();
    const maxLeft = Math.max(
      VIEWPORT_MARGIN,
      window.innerWidth - bounds.width - VIEWPORT_MARGIN,
    );
    const maxTop = Math.max(
      VIEWPORT_MARGIN,
      window.innerHeight - bounds.height - VIEWPORT_MARGIN,
    );

    setPosition({
      left: Math.min(Math.max(VIEWPORT_MARGIN, x), maxLeft),
      top: Math.min(Math.max(VIEWPORT_MARGIN, y), maxTop),
    });
  }, [activeSubmenu, payload, x, y]);

  if (!payload) return null;

  const handleMenuContext = (event: MouseEvent) => {
    event.preventDefault();
  };

  const handleBack = () => {
    setActiveSubmenu(null);
  };

  const handleItemClick = (item: ContextMenuItemPayload) => {
    if (item.groups && item.groups.length > 0) {
      setActiveSubmenu(item);
      return;
    }

    onPick(item.action);
  };

  return (
    <div
      className="fixed z-50 box-border max-h-100 w-55 select-none overflow-y-auto rounded-2 border border-ant-border-secondary bg-ant-elevated p-1 shadow-lg"
      onContextMenu={handleMenuContext}
      ref={menuRef}
      role="menu"
      style={position}
    >
      {activeSubmenu ? (
        <>
          <button
            className="flex h-8 w-full items-center gap-2 rounded-1.5 px-2 text-left text-ant-secondary text-sm hover:bg-ant-fill-secondary"
            onClick={handleBack}
            type="button"
          >
            <i aria-hidden="true" className="i-lucide:chevron-left size-4" />
            <span className="truncate">{activeSubmenu.label}</span>
          </button>
          <div className="my-1 h-px bg-ant-split" />
          {activeSubmenu.groups?.map((group) => {
            const handleGroupClick = () => {
              onPick(activeSubmenu.action, group.id);
            };

            return (
              <button
                aria-checked={group.checked}
                className="flex h-8 w-full items-center gap-2 rounded-1.5 px-2 text-left text-ant-text text-sm hover:bg-ant-fill-secondary"
                key={group.id}
                onClick={handleGroupClick}
                role="menuitemradio"
                type="button"
              >
                <i
                  aria-hidden="true"
                  className={cn("size-4", {
                    "i-lucide:check": group.checked,
                  })}
                />
                <span className="min-w-0 flex-1 truncate">{group.label}</span>
              </button>
            );
          })}
        </>
      ) : (
        payload.groups.map((group, groupIndex) => {
          return (
            // biome-ignore lint/suspicious/noArrayIndexKey: backend-defined group order is stable.
            <Fragment key={groupIndex}>
              {groupIndex > 0 ? (
                <div className="my-1 h-px bg-ant-split" />
              ) : null}
              {group.map((item) => {
                const handleClick = () => {
                  handleItemClick(item);
                };

                return (
                  <button
                    className={cn(
                      "flex h-8 w-full items-center gap-2 rounded-1.5 px-2 text-left text-ant-text text-sm hover:bg-ant-fill-secondary",
                      {
                        "text-ant-error": item.action === "delete",
                      },
                    )}
                    key={item.action}
                    onClick={handleClick}
                    role="menuitem"
                    type="button"
                  >
                    <span className="min-w-0 flex-1 truncate">{item.label}</span>
                    {item.accelerator ? (
                      <span className="shrink-0 text-ant-quaternary text-xs">
                        {formatShortcutDisplay(item.accelerator)}
                      </span>
                    ) : null}
                    {item.groups && item.groups.length > 0 ? (
                      <i
                        aria-hidden="true"
                        className="i-lucide:chevron-right size-4 shrink-0 text-ant-quaternary"
                      />
                    ) : null}
                  </button>
                );
              })}
            </Fragment>
          );
        })
      )}
    </div>
  );
};

export default ClipboardContextMenu;
