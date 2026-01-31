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
import { clipboardStore } from "@/stores/clipboard";
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

      // 搜索框默认聚焦
      if (search.defaultFocus) {
        inputRef.current?.focus();
      } else {
        inputRef.current?.blur();
      }
    },
  });

  useKeyPress(PRESET_SHORTCUT.SEARCH, () => {
    inputRef.current?.focus();
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

  // 监听全局输入事件（来自后端 rdev 捕获的按键）
  useTauriListen<{ key: string }>("input-event", ({ payload }) => {
    const { key } = payload;

    if (key === "Backspace") {
      setValue((prev) => (prev ? prev.slice(0, -1) : ""));
    } else if (key === "Delete") {
      setValue("");
    } else {
      setValue((prev) => (prev || "") + key);
    }
  });

  useMount(() => {
    if (clipboardStore.search.defaultFocus) {
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
        ref={inputRef}
        size="small"
        value={value}
      />
    </div>
  );
};

export default SearchInput;
