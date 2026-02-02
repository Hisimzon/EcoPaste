import { pinyin } from "pinyin-pro";

/**
 * 从文本中提取中文并生成拼音索引
 * @param text 原始文本
 * @param maxLength 最大处理的中文字符数，避免长文本导致数据膨胀
 * @returns 拼音字符串（全拼 + 首字母），如果没有中文则返回空字符串
 */
export function generatePinyinIndex(
  text: string | undefined,
  maxLength = 200,
): string {
  if (!text) return "";

  // 只提取中文字符
  const chineseChars = text.match(/[\u4e00-\u9fa5]/g);
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
 * @returns 合并后的搜索文本（原文 + 拼音）
 */
export function appendPinyinToSearch(searchText: string | undefined): string {
  if (!searchText) return "";

  const pinyinIndex = generatePinyinIndex(searchText);

  if (!pinyinIndex) return searchText;

  return `${searchText} ${pinyinIndex}`;
}
