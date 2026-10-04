// Paragraph splitting shared by the Studio screen and its dev mock. Mirrors
// `split_paragraphs` in src-tauri/src/studio.rs: blocks separated by blank
// lines, or each non-empty line when the script has no blank lines.
export function paragraphsOf(text: string): string[] {
  const blocks: string[][] = [[]];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line) {
      if (blocks[blocks.length - 1].length) blocks.push([]);
    } else {
      blocks[blocks.length - 1].push(line);
    }
  }
  const nonEmpty = blocks.filter((b) => b.length);
  if (nonEmpty.length === 1 && nonEmpty[0].length > 1) return nonEmpty[0];
  return nonEmpty.map((b) => b.join(" "));
}
