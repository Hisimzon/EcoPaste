import { useBoolean, useKeyPress, useMount } from "ahooks";
import type { InputRef } from "antd";
import { Input } from "antd";
import {
  type FC,
  type HTMLAttributes,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import UnoIcon from "@/components/UnoIcon";
import { PRESET_SHORTCUT } from "@/constants";
import { useTauriFocus } from "@/hooks/useTauriFocus";
import { useTauriListen } from "@/hooks/useTauriListen";
import { hideWindow } from "@/plugins/window";
import { clipboardStore } from "@/stores/clipboard";
import { isWin } from "@/utils/is";
import { MainContext } from "../..";

const SearchInput: FC<HTMLAttributes<HTMLDivElement>> = (props) => {
  const { rootState } = useContext(MainContext);
  const inputRef = useRef<InputRef>(null);
  const [value, setValue] = useState<string>();
  const [isComposition, { setTrue, setFalse }] = useBoolean();
  const { t } = useTranslation();

  useEffect(() => {
    if (isComposition) return;

    rootState.search = value;
  }, [value, isComposition]);

  useTauriFocus({
    onBlur() {
      const { search } = clipboardStore;

      // 搜索框自动清空
      if (search.autoClear) {
        setValue(void 0);
      }
    },
    onFocus() {
      const { search } = clipboardStore;

      // 非 Windows 平台：搜索框默认聚焦
      if (!isWin && search.defaultFocus) {
        inputRef.current?.focus();
      }
    },
  });

  // 非 Windows 平台：快捷键聚焦搜索框
  useKeyPress(PRESET_SHORTCUT.SEARCH, () => {
    if (!isWin) {
      inputRef.current?.focus();
    }
  });

  useKeyPress(
    ["enter", "uparrow", "downarrow"],
    () => {
      inputRef.current?.blur();
    },
    {
      target: inputRef.current?.input,
    },
  );

  // Windows 平台：通过 rdev 捕获键盘输入
  useTauriListen<{ code: string }>("dispatch-event", ({ payload }) => {
    if (!isWin) return;

    if (payload.code === "Escape") {
      hideWindow();
      return;
    } else if (payload.code === "Backspace") {
      // Backspace 删除搜索框最后一个字符
      setValue((prev) => {
        if (!prev || prev.length === 0) return prev;
        return prev.slice(0, -1) || undefined;
      });
    }
  });

  // Windows 平台：监听 rdev 捕获的可打印字符
  useTauriListen<{ char: string }>("search-input", ({ payload }) => {
    if (!isWin) return;

    setValue((prev) => (prev || "") + payload.char);
  });

  useMount(() => {
    // 非 Windows 平台：默认聚焦搜索框
    if (!isWin && clipboardStore.search.defaultFocus) {
      inputRef.current?.focus();
    }
  });

  return (
    <div {...props}>
      <Input
        allowClear
        autoCorrect="off"
        onChange={(event) => {
          setValue(event.target.value);
        }}
        onCompositionEnd={setFalse}
        onCompositionStart={setTrue}
        placeholder={t("clipboard.hints.search_placeholder")}
        prefix={<UnoIcon name="i-lucide:search" />}
        readOnly={isWin}
        ref={inputRef}
        size="small"
        // Windows 平台禁用输入框交互，通过 rdev 捕获输入
        style={isWin ? { cursor: "default" } : undefined}
        value={value}
      />
    </div>
  );
};

export default SearchInput;
