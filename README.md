# tile-image-viewer
 
`tile-image-viewer` is a program for viewing super resolution images. It works by only rendering parts of an image that are on your screen, at a resolution that won't overwhelm your computer. To slice your own image into tiles so `tile-image-viewer` can view it, see [tile-processor](https://github.com/sloganking/tile-processor).

An example of how a large image can be rendered at various levels of details (LODs)
![](https://raw.githubusercontent.com/banesullivan/localtileserver/main/imgs/tile-diagram.gif)


 ## `tile-image-viewer` includes 
- Rendering tiles of various sizes (not at once).
- Rendering various tile LODs. So only between 1-4x the resolution of your screen in pixels, will ever be cached in memory.
- Occlusion culling. So you will never render more tiles than are necessary to fill your screen.
- Tile Debug view. Displays a red box around each rendered tile.
- Debug stats in the top left. Including
  - fps
  - zoom multiplier
  - Current tile LOD
  - How many tiles are rendered on screen
  - the coordinates of the pixel the mouse is over. Where one pixel of the full resolution image is one coordinate
