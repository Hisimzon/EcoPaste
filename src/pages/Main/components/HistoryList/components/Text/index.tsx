import { Flex } from "antd";
import clsx from "clsx";
import { type CSSProperties, type FC, useContext } from "react";
import { Marker } from "react-mark.js";
import { useSnapshot } from "valtio";
import { MainContext } from "@/pages/Main";
import { clipboardStore } from "@/stores/clipboard";
import type { DatabaseSchemaHistory } from "@/types/database";
import { normalizeTextThreshold } from "@/utils/threshold";

const Text: FC<DatabaseSchemaHistory<"text">> = (props) => {
  const { value, subtype } = props;
  const { rootState } = useContext(MainContext);
  const { content } = useSnapshot(clipboardStore);
  // 使用配置阈值控制预览长度
  const textThreshold = normalizeTextThreshold(content.textThreshold);
  // 大文本仅渲染前缀，保障主线程响应
  const previewValue =
    value.length > textThreshold ? value.slice(0, textThreshold) : value;

  const renderMarker = () => {
    // 无搜索关键词时不走 Marker，减少高亮计算
    if (!rootState.search) {
      return previewValue;
    }

    return <Marker mark={rootState.search}>{previewValue}</Marker>;
  };

  const renderColor = () => {
    const className = "absolute rounded-full";
    const style: CSSProperties = {
      background: value,
    };

    return (
      <Flex align="center" gap="small">
        <div className="relative h-5.5 min-w-5.5">
          <span
            className={clsx(className, "inset-0 opacity-50")}
            style={style}
          />

          <span className={clsx(className, "inset-0.5")} style={style} />
        </div>

        {renderMarker()}
      </Flex>
    );
  };

  const renderContent = () => {
    if (subtype === "color") {
      return renderColor();
    }

    return renderMarker();
  };

  return <div className="line-clamp-4">{renderContent()}</div>;
};

export default Text;
