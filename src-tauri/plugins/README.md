# Pin-window AI badge

This is not a PixPin/Snipaste plugin package.

Anya itself detects pin windows from those apps and draws an "AI" badge at the
bottom-right of each pin. Clicking the badge opens the Anya chat overlay with
the pin image attached.

Optional local API while Anya is running:

  POST http://127.0.0.1:18480/api/ask/image
  {"image":"data:image/png;base64,..."}
  or {"path":"C:/absolute/image.png"}
