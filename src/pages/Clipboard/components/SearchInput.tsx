import { Input, type InputProps, type InputRef } from "antd";
import type { ChangeEvent, CompositionEvent, FC } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import KeyHint from "@/components/KeyHint";
import { TAURI_EVENT } from "@/constants/events";
import { useTauriListen } from "@/hooks/useTauriListen";
import { prepareClipboardWindowEditableFocus } from "@/hooks/useClipboardWindowEditableFocus";
import { isWinClipboardWindow } from "@/utils/is";

interface SearchInputEvent {
  action: "append" | "backspace" | "replace";
  text?: string;
}

interface SearchInputProps
  extends Omit<InputProps, "defaultValue" | "onChange" | "prefix" | "value"> {
  blurToken?: number;
  clearToken?: number;
  focusToken?: number;
  onValueChange?: (value: string) => void;
}

/**
 * Windows 剪贴板窗口不获取焦点：搜索内容由 Rust 低级键盘钩子驱动。
 * macOS 继续使用 WebView 原生输入和 IME。
 */
const SearchInput: FC<SearchInputProps> = (props) => {
  const {
    blurToken = 0,
    clearToken = 0,
    focusToken = 0,
    onClear,
    onCompositionEnd,
    onCompositionStart,
    onValueChange,
    ...rest
  } = props;
  const isWindowsNoFocus = isWinClipboardWindow();
  const inputRef = useRef<InputRef>(null);
  const composingRef = useRef(false);
  const replaceNextRef = useRef(false);
  const [value, setValue] = useState("");

  const updateValue = useCallback(
    (nextValue: string) => {
      setValue(nextValue);
      onValueChange?.(nextValue);
    },
    [onValueChange],
  );

  useTauriListen<SearchInputEvent>(TAURI_EVENT.SEARCH_INPUT, (event) => {
    if (!isWindowsNoFocus) return;

    const { action, text = "" } = event.payload;
    if (action === "replace") {
      replaceNextRef.current = true;
      return;
    }

    if (action === "append") {
      const nextValue = replaceNextRef.current ? text : `${value}${text}`;
      replaceNextRef.current = false;
      updateValue(nextValue);
      return;
    }

    const nextValue = replaceNextRef.current
      ? ""
      : Array.from(value).slice(0, -1).join("");
    replaceNextRef.current = false;
    updateValue(nextValue);
  });

  const focusSearch = useCallback(async () => {
    if (!inputRef.current) return;

    if (isWindowsNoFocus) {
      replaceNextRef.current = true;
      return;
    }

    await prepareClipboardWindowEditableFocus();
    inputRef.current.focus({ cursor: "all" });
  }, [isWindowsNoFocus]);

  useEffect(() => {
    if (blurToken <= 0 || isWindowsNoFocus) return;

    inputRef.current?.blur();
  }, [blurToken, isWindowsNoFocus]);

  useEffect(() => {
    if (clearToken <= 0) return;

    replaceNextRef.current = false;
    updateValue("");
  }, [clearToken, updateValue]);

  useEffect(() => {
    if (focusToken <= 0) return;

    const frame = requestAnimationFrame(() => {
      void focusSearch();
    });

    return () => {
      cancelAnimationFrame(frame);
    };
  }, [focusToken, focusSearch]);

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    if (composingRef.current) return;

    updateValue(event.target.value);
  };

  const handleCompositionStart = (
    event: CompositionEvent<HTMLInputElement>,
  ) => {
    composingRef.current = true;
    onCompositionStart?.(event);
  };

  const handleCompositionEnd = (event: CompositionEvent<HTMLInputElement>) => {
    composingRef.current = false;
    onCompositionEnd?.(event);
    updateValue(event.currentTarget.value);
  };

  const handleClear = () => {
    replaceNextRef.current = false;
    updateValue("");
    onClear?.();
  };

  return (
    <Input
      autoCapitalize="off"
      autoCorrect="off"
      data-allow-global-keyboard="true"
      onChange={handleChange}
      onClear={handleClear}
      onCompositionEnd={handleCompositionEnd}
      onCompositionStart={handleCompositionStart}
      prefix={
        <KeyHint
          hintKey="F"
          iconName="i-lucide:search"
          onKeyPress={focusSearch}
        />
      }
      ref={inputRef}
      spellCheck={false}
      {...rest}
      onMouseDown={
        isWindowsNoFocus
          ? (event) => {
              event.preventDefault();
            }
          : rest.onMouseDown
      }
      readOnly={isWindowsNoFocus}
      tabIndex={isWindowsNoFocus ? -1 : rest.tabIndex}
      value={value}
    />
  );
};

export default SearchInput;
