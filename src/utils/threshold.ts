// 大文本处理阈值（单位：字符）
export const TEXT_THRESHOLD_DEFAULT = 4096;
// 合理范围下限，避免过小导致搜索/预览体验下降
export const TEXT_THRESHOLD_MIN = 512;
// 合理范围上限，避免过大导致性能问题
export const TEXT_THRESHOLD_MAX = 20000;

/**
 * 规范化大文本阈值，确保落在合理范围内
 */
export const normalizeTextThreshold = (value?: number | null) => {
  // 空值使用默认阈值
  if (value == null) {
    return TEXT_THRESHOLD_DEFAULT;
  }

  const numeric = Number(value);

  if (!Number.isFinite(numeric)) {
    return TEXT_THRESHOLD_DEFAULT;
  }

  const rounded = Math.round(numeric);

  return Math.min(TEXT_THRESHOLD_MAX, Math.max(TEXT_THRESHOLD_MIN, rounded));
};
