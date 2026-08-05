import { Input, type InputProps, type InputRef } from "antd";
import type { ChangeEvent, CompositionEvent, FC } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import KeyHint from "@/components/KeyHint";
import { TAURI_EVENT } from "@/constants/events";
import { prepareClipboardWindowEditableFocus } from "@/hooks/useClipboardWindowEditableFocus";
import { useTauriListen } from "@/hooks/useTauriListen";
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
 * Windows 快捷键呼出时由 Rust 低级键盘钩子驱动搜索；用户手动点击输入框后临时启用原生输入和 IME。
 * macOS 始终使用 WebView 原生输入。
 */
const SearchInput: FC<SearchInputProps> = (props) => {
  const {
    blurToken = 0,
    clearToken = 0,
    focusToken = 0,
    onBlur,
    onClear,
    onCompositionEnd,
    onCompositionStart,
    onFocus,
    onMouseDown,
    onValueChange,
    ...rest
  } = props;
  const isWindowsNoFocus = isWinClipboardWindow();
  const inputRef = useRef<InputRef>(null);
  const composingRef = useRef(false);
  const replaceNextRef = useRef(false);
  const [manualEditing, setManualEditing] = useState(false);
  const [value, setValue] = useState("");
  const usesVirtualInput = isWindowsNoFocus && !manualEditing;

  const updateValue = useCallback(
    (nextValue: string) => {
      setValue(nextValue);
      onValueChange?.(nextValue);
    },
    [onValueChange],
  );

  useTauriListen<SearchInputEvent>(TAURI_EVENT.SEARCH_INPUT, (event) => {
    if (!usesVirtualInput) return;

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

    if (usesVirtualInput) {
      replaceNextRef.current = true;
      return;
    }

    await prepareClipboardWindowEditableFocus();
    inputRef.current.focus({ cursor: "all" });
  }, [usesVirtualInput]);

  useEffect(() => {
    if (blurToken <= 0 || usesVirtualInput) return;

    inputRef.current?.blur();
  }, [blurToken, usesVirtualInput]);

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

  const activateManualSearch = async () => {
    await prepareClipboardWindowEditableFocus();

    requestAnimationFrame(() => {
      inputRef.current?.focus({ cursor: "end" });
    });
  };

  const handleMouseDown: NonNullable<InputProps["onMouseDown"]> = (event) => {
    onMouseDown?.(event);

    if (!isWindowsNoFocus) return;
    if (event.defaultPrevented) return;
    if (event.button !== 0 || !event.nativeEvent.isTrusted) return;

    event.preventDefault();
    setManualEditing(true);
    void activateManualSearch();
  };

  const handleFocus: NonNullable<InputProps["onFocus"]> = (event) => {
    if (isWindowsNoFocus) setManualEditing(true);

    onFocus?.(event);
  };

  const handleBlur: NonNullable<InputProps["onBlur"]> = (event) => {
    if (isWindowsNoFocus) setManualEditing(false);

    onBlur?.(event);
  };

  return (
    <Input
      autoCapitalize="off"
      autoCorrect="off"
      data-allow-global-keyboard="true"
      onBlur={handleBlur}
      onChange={handleChange}
      onClear={handleClear}
      onCompositionEnd={handleCompositionEnd}
      onCompositionStart={handleCompositionStart}
      onFocus={handleFocus}
      onMouseDown={handleMouseDown}
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
      readOnly={usesVirtualInput}
      tabIndex={usesVirtualInput ? -1 : rest.tabIndex}
      value={value}
    />
  );
};

export default SearchInput;
