import { pinyin } from "pinyin-pro";
import { normalizeTextThreshold } from "@/utils/threshold";

export const NOTE_PINYIN_MARKER = "\x1FNOTE\x1F";
export const SEARCH_PINYIN_MARKER = "\x1FPINYIN\x1F";

export function stripNotePinyinMarker(value: string | undefined): string {
  if (!value) return "";

  const index = value.indexOf(NOTE_PINYIN_MARKER);

  if (index === -1) return value;

  return value.slice(0, index).trimEnd();
}

export function stripSearchIndex(
  value: string | undefined,
  textThreshold?: number,
): string {
  const base = stripNotePinyinMarker(value).trimEnd();
  const maxLength = normalizeTextThreshold(textThreshold);

  if (!base) return "";

  const markerIndex = base.indexOf(SEARCH_PINYIN_MARKER);

  if (markerIndex !== -1) {
    return base.slice(0, markerIndex).trimEnd();
  }

  // 超过阈值直接返回，避免 split + 逐段比对的开销
  if (base.length > maxLength) {
    return base;
  }

  const segments = base.split(/\s+/).filter(Boolean);

  if (segments.length < 2) return base;

  for (let i = 1; i < segments.length; i++) {
    const content = segments.slice(0, i).join(" ");
    const maybePinyin = segments.slice(i).join(" ");
    const expected = generatePinyinIndex(content);

    if (expected && maybePinyin === expected) {
      return content.trimEnd();
    }
  }

  return base;
}

/**
 * 从文本中提取中文并生成拼音索引
 * @param text 原始文本
 * @param maxLength 最大处理的中文字符数，避免长文本导致数据膨胀
 * @param textThreshold 大文本阈值，控制扫描长度
 * @returns 拼音字符串（全拼 + 首字母），如果没有中文则返回空字符串
 */
export function generatePinyinIndex(
  text: string | undefined,
  maxLength = 200,
  textThreshold?: number,
): string {
  if (!text) return "";

  // 只提取中文字符（扫描长度做上限，避免大文本阻塞）
  const maxScanLength = normalizeTextThreshold(textThreshold);
  const scanText =
    text.length > maxScanLength ? text.slice(0, maxScanLength) : text;
  const chineseChars = scanText.match(/[\u4e00-\u9fa5]/g);
  if (!chineseChars || chineseChars.length === 0) return "";

  // 限制长度，避免数据膨胀
  const chineseText = chineseChars.slice(0, maxLength).join("");

  // 生成全拼（无声调，无空格）
  const fullPinyin = pinyin(chineseText, {
    toneType: "none",
    type: "array",
  }).join("");

  // 生成首字母
  const firstLetters = pinyin(chineseText, {
    pattern: "first",
    toneType: "none",
    type: "array",
  }).join("");

  return `${fullPinyin} ${firstLetters}`;
}

/**
 * 将原始搜索文本与拼音索引合并
 * @param searchText 原始搜索文本
 * @param textThreshold 大文本阈值，控制拼音索引扫描长度
 * @returns 合并后的搜索文本（原文 + 拼音）
 */
export function appendPinyinToSearch(
  searchText: string | undefined,
  textThreshold?: number,
): string {
  const maxLength = normalizeTextThreshold(textThreshold);
  const baseSearch = stripSearchIndex(searchText, maxLength).trimEnd();

  if (!baseSearch) return "";

  const scanText =
    baseSearch.length > maxLength ? baseSearch.slice(0, maxLength) : baseSearch;
  const pinyinIndex = generatePinyinIndex(scanText, 200, maxLength);

  if (!pinyinIndex) return baseSearch;

  return `${baseSearch} ${SEARCH_PINYIN_MARKER} ${pinyinIndex}`;
}
