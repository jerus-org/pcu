+++
title = "Update and Release"
template = "series.html"
sort_by = "slug"
transparent = true

[extra]
series = true

# Introduction.
[extra.series_intro_templates]
next_only = """
Welcome to $SERIES_HTML_LINK! 
$SERIES_PAGES_OLIST

This $SERIES_PAGES_NUMBER-part series outlines the workflow and tools I've implemented to ensure that the open-source software I produce is updated and released weekly, addressing any security issues that may arise in dependencies.

Next: $NEXT_HTML_LINK - $NEXT_DESCRIPTION
"""

middle = """
📚 Part $SERIES_PAGE_INDEX of $SERIES_PAGES_NUMBER in $SERIES_HTML_LINK

$SERIES_PAGES_OLIST

- Previous: $PREV_HTML_LINK
- Next: $NEXT_HTML_LINK
"""

prev_only = """
Welcome to the final part of $SERIES_HTML_LINK!

New here? Start with $FIRST_HTML_LINK to build a strong foundation.

$SERIES_PAGES_OLIST

- Previous: $PREV_HTML_LINK
"""

# Fallback template.
default = "This article is part of the $SERIES_HTML_LINK series."

# Outro.
[extra.series_outro_templates]
next_only = """
$SERIES_PAGES_OLIST

Thanks for reading! 🙌

Continue your journey with $NEXT_HTML_LINK, where $NEXT_DESCRIPTION
Or check out the complete [$SERIES_TITLE]($SERIES_PERMALINK) series outline.
"""

middle = """
---
📝 Series Navigation

- Previous: $PREV_HTML_LINK
- Next: $NEXT_HTML_LINK
- [Series Overview]($SERIES_PERMALINK)
"""

prev_only = """
🎉 Congratulations! You've completed $SERIES_HTML_LINK.

Want to review? Here's where we started: $FIRST_HTML_LINK
Or check what we just covered in $PREV_HTML_LINK.
"""

# Fallback.
default = """
---
This article is part $SERIES_PAGE_INDEX of $SERIES_PAGES_NUMBER in $SERIES_HTML_LINK.
"""


+++

This series outlines the workflow and tools I've implemented to ensure that the open-source software I produce is updated and released weekly, addressing any security issues that may arise in dependencies.
