# TileView
 
`TileView` is a program for viewing super resolution images. It works by only rendering parts of an image that are on your screen, at a resolution that won't overwhelm your computer. To slice your own image into tiles so `TileView` can view it, see [tile-processor](https://github.com/sloganking/tile-processor).

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


