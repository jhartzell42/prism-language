-- Strip "Chapter N — " / "Chapter N: " from level-1 headings so LaTeX numbers chapters;
-- mark the introduction unnumbered.
function Header(h)
  if h.level == 1 then
    local s = pandoc.utils.stringify(h.content)
    local rest = s:match("^Chapter %d+%s*[—:%-]+%s*(.+)$")
    if rest then
      h.content = pandoc.Inlines(pandoc.Str(rest))
      -- keep inline markup lost: titles are plain text
    end
    if s == "Introduction" then
      h.classes:insert("unnumbered")
    end
  end
  return h
end
