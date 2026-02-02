import { useBoolean } from "ahooks";
import { Form, Input, type InputRef, Modal } from "antd";
import { find } from "es-toolkit/compat";
import { t } from "i18next";
import {
  forwardRef,
  useContext,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { updateHistory } from "@/database/history";
import { MainContext } from "@/pages/Main";
import { enterInputMode, exitInputMode } from "@/plugins/window";
import { clipboardStore } from "@/stores/clipboard";
import type { DatabaseSchemaHistory } from "@/types/database";
import { isWin } from "@/utils/is";
import { generatePinyinIndex } from "@/utils/pinyin";

// 用于分隔原始搜索内容和备注拼音的标记
const NOTE_PINYIN_MARKER = "\x1FNOTE\x1F";

export interface NoteModalRef {
  open: (id: string) => void;
}

interface FormFields {
  note: string;
}

const NoteModal = forwardRef<NoteModalRef>((_, ref) => {
  const { rootState } = useContext(MainContext);
  const [open, { toggle }] = useBoolean();
  const [item, setItem] = useState<DatabaseSchemaHistory>();
  const [form] = Form.useForm<FormFields>();
  const inputRef = useRef<InputRef>(null);

  useImperativeHandle(ref, () => ({
    open: (id) => {
      const findItem = find(rootState.list, { id });

      form.setFieldsValue({
        note: findItem?.note,
      });

      setItem(findItem);

      toggle();
    },
  }));

  const handleOk = async () => {
    const { note } = form.getFieldsValue();

    if (item) {
      const { id, favorite, search } = item;

      item.note = note;

      // 构建包含备注拼音的搜索字段
      // 先移除之前的备注拼音部分（如果有）
      const baseSearch = (search || "").split(NOTE_PINYIN_MARKER)[0].trimEnd();

      let newSearch = baseSearch;
      if (note) {
        // 添加备注内容和其拼音到搜索字段
        const notePinyin = generatePinyinIndex(note);
        const notePart = notePinyin ? `${note} ${notePinyin}` : note;
        newSearch = `${baseSearch} ${NOTE_PINYIN_MARKER} ${notePart}`;
      }

      item.search = newSearch;

      updateHistory(id, { note, search: newSearch });

      if (clipboardStore.content.autoFavorite && !favorite) {
        item.favorite = true;

        updateHistory(id, { favorite: true });
      }
    }

    toggle();
  };

  const handleAfterOpenChange = (open: boolean) => {
    // Windows 不抢占焦点模式：Modal 打开时进入输入模式，关闭时退出
    if (isWin) {
      if (open) {
        enterInputMode();
      } else {
        exitInputMode();
      }
    }

    if (!open) return;

    inputRef.current?.focus();
  };

  return (
    <Modal
      afterOpenChange={handleAfterOpenChange}
      centered
      forceRender
      onCancel={toggle}
      onOk={handleOk}
      open={open}
      title={t("component.note_modal.label.note")}
    >
      <Form
        form={form}
        initialValues={{ note: item?.note }}
        onFinish={handleOk}
      >
        <Form.Item className="mb-0!" name="note">
          <Input
            autoComplete="off"
            placeholder={t("component.note_modal.hints.input_note")}
            ref={inputRef}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
});

export default NoteModal;
