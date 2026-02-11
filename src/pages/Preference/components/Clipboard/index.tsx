import { InputNumber } from "antd";
import { useTranslation } from "react-i18next";
import { useSnapshot } from "valtio";
import ProList from "@/components/ProList";
import ProListItem from "@/components/ProListItem";
import ProSwitch from "@/components/ProSwitch";
import { clipboardStore } from "@/stores/clipboard";
import {
  normalizeTextThreshold,
  TEXT_THRESHOLD_DEFAULT,
  TEXT_THRESHOLD_MAX,
  TEXT_THRESHOLD_MIN,
} from "@/utils/threshold";
import AudioSettings from "./components/AudioSettings";
import AutoPaste from "./components/AutoPaste";
import OperationButton from "./components/OperationButton";
import SearchPosition from "./components/SearchPosition";
import WindowPosition from "./components/WindowPosition";

const ClipboardSettings = () => {
  const { window, search, content } = useSnapshot(clipboardStore);
  const { t } = useTranslation();

  return (
    <>
      <ProList header={t("preference.clipboard.window_settings.title")}>
        <WindowPosition />

        <ProSwitch
          description={t("preference.clipboard.window_settings.hints.back_top")}
          onChange={(value) => {
            clipboardStore.window.backTop = value;
          }}
          title={t("preference.clipboard.window_settings.label.back_top")}
          value={window.backTop}
        />

        <ProSwitch
          onChange={(value) => {
            clipboardStore.window.showAll = value;
          }}
          title={t("preference.clipboard.window_settings.label.show_all")}
          value={window.showAll}
        />
      </ProList>

      <AudioSettings />

      <ProList header={t("preference.clipboard.search_box_settings.title")}>
        <SearchPosition key={1} />

        <ProSwitch
          description={t(
            "preference.clipboard.search_box_settings.hints.default_focus",
          )}
          onChange={(value) => {
            clipboardStore.search.defaultFocus = value;
          }}
          title={t(
            "preference.clipboard.search_box_settings.label.default_focus",
          )}
          value={search.defaultFocus}
        />

        <ProSwitch
          description={t(
            "preference.clipboard.search_box_settings.hints.auto_clear",
          )}
          onChange={(value) => {
            clipboardStore.search.autoClear = value;
          }}
          title={t("preference.clipboard.search_box_settings.label.auto_clear")}
          value={search.autoClear}
        />
      </ProList>

      <ProList header={t("preference.clipboard.content_settings.title")}>
        <AutoPaste />

        <ProSwitch
          description={t(
            "preference.clipboard.content_settings.hints.copy_as_plain",
          )}
          onChange={(value) => {
            clipboardStore.content.copyPlain = value;
          }}
          title={t("preference.clipboard.content_settings.label.copy_as_plain")}
          value={content.copyPlain}
        />

        <ProSwitch
          description={t(
            "preference.clipboard.content_settings.hints.paste_as_plain",
          )}
          onChange={(value) => {
            clipboardStore.content.pastePlain = value;
          }}
          title={t(
            "preference.clipboard.content_settings.label.paste_as_plain",
          )}
          value={content.pastePlain}
        />

        <OperationButton />

        <ProSwitch
          description={t(
            "preference.clipboard.content_settings.hints.auto_favorite",
          )}
          onChange={(value) => {
            clipboardStore.content.autoFavorite = value;
          }}
          title={t("preference.clipboard.content_settings.label.auto_favorite")}
          value={content.autoFavorite}
        />

        <ProSwitch
          description={t(
            "preference.clipboard.content_settings.hints.delete_confirm",
          )}
          onChange={(value) => {
            clipboardStore.content.deleteConfirm = value;
          }}
          title={t(
            "preference.clipboard.content_settings.label.delete_confirm",
          )}
          value={content.deleteConfirm}
        />

        <ProSwitch
          description={t(
            "preference.clipboard.content_settings.hints.auto_sort",
          )}
          onChange={(value) => {
            clipboardStore.content.autoSort = value;
          }}
          title={t("preference.clipboard.content_settings.label.auto_sort")}
          value={content.autoSort}
        />

        <ProSwitch
          description={t(
            "preference.clipboard.content_settings.hints.show_original_content",
          )}
          onChange={(value) => {
            clipboardStore.content.showOriginalContent = value;
          }}
          title={t(
            "preference.clipboard.content_settings.label.show_original_content",
          )}
          value={content.showOriginalContent}
        />

        {/* 大文本阈值，限制在合理范围内 */}
        <ProListItem
          description={t(
            "preference.clipboard.content_settings.hints.text_threshold",
            {
              default: TEXT_THRESHOLD_DEFAULT,
              max: TEXT_THRESHOLD_MAX,
              min: TEXT_THRESHOLD_MIN,
            },
          )}
          title={t(
            "preference.clipboard.content_settings.label.text_threshold",
          )}
        >
          <InputNumber
            addonAfter={t(
              "preference.clipboard.content_settings.label.text_threshold_unit",
            )}
            className="w-30"
            max={TEXT_THRESHOLD_MAX}
            min={TEXT_THRESHOLD_MIN}
            onChange={(value) => {
              clipboardStore.content.textThreshold = normalizeTextThreshold(
                value ?? TEXT_THRESHOLD_DEFAULT,
              );
            }}
            step={256}
            value={content.textThreshold}
          />
        </ProListItem>
      </ProList>
    </>
  );
};

export default ClipboardSettings;
