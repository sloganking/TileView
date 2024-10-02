# TileView
 
`TileView` is a program for viewing super resolution images. It works by only rendering parts of an image that are on your screen, at a resolution that won't overwhelm your computer. You can run `TileView` on a directory containing a tileset, or on a standard image file. If you run `TileView` on an image file, it will use [tile-processor](https://github.com/sloganking/tile-processor) to convert the image to tiles in a tmp directory before viewing it.

## TileView in debug mode

https://github.com/user-attachments/assets/08aa6c8f-e092-490a-9338-9302d6d5a9a0

Image source: [Cosmic Cliffs (14,575 x 8,441)](https://webbtelescope.org/contents/media/images/2022/031/01G77PKB8NKR7S8Z6HBXMYATGJ?page=1&keyword=pillar)


An example of how a large image can be rendered at various levels of details (LODs)
![](https://raw.githubusercontent.com/banesullivan/localtileserver/main/imgs/tile-diagram.gif)


 ## `TileView` includes 

### Features
- Rendering tiles of various sizes (not at once).
- Tile Debug view. Displays a red box around each rendered tile.
- Debug stats in the top left. Including
  - fps
  - zoom multiplier
  - Current tile LOD
  - How many tiles are rendered on screen
  - the coordinates of the pixel the mouse is over. Where one pixel of the full resolution image is one coordinate
### Optimizations
- Rendering various tile LODs. So only between 1-4x the resolution of your screen in pixels, will ever be cached in memory and rendered.
- Occlusion culling. So you will never render more tiles than are necessary to fill your screen.
- Asyncronous tile retrieval. Tiles are retrieved and decompressed, only so long as there is enough time to decompress a tile before the application must render the next frame.
- Advanced tile caching, tiles off screen are immeditely removed from memory, however tiles on screen from a different LOD than is currently desired, are rendered and not removed from tile cache until all requested tiles from the current LOD are rendered. This allows zooming in and out without the map disapearing when your view changes tile layers.


