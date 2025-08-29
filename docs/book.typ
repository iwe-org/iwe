#import "@preview/cmarker:0.1.6"

#cmarker.render(
  read("book.md"),
  scope: (image: (path, alt: none) => image(path, alt: alt))
)
