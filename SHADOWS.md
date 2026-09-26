# Rendering stability

## Directional shadows

The renderer draws terrain and tree geometry into two 2048 by 2048 depth-map
layers. The near layer reaches 48 world units; the far layer reaches 160. They
blend across the 32 to 48 unit range, and the far layer fades over its last 32
units. The light-space center follows the camera and snaps to a shadow texel.
Its basis follows the sun's analytic orbit tangent, so it remains continuous
through noon and day wrap. Shadow texels still rotate with the sun, as the
projected light direction changes.

The shader uses a 4 by 4 separable tent filter. For each tap it corrects the
comparison depth to the receiver's geometric plane. That correction is
ill-conditioned when a face is nearly edge-on to the sun, so the denominator
is bounded and the correction and shadow contribution fade smoothly at grazing
angles. The shadow rasterizer also applies a small constant and slope-scaled
depth bias, capped to keep steep faces from being pushed far from their actual
depth. These controls trade self-shadow acne against detached shadows and
should stay small.

Shadow strength fades smoothly from zero at six degrees of sun elevation to
full strength at twelve degrees. This keeps extremely long low-sun shadows
from consuming unbounded map coverage. Terrain and spruce trees cast shadows;
the ground plants do not. Spruce leaves use the same alpha cutoff in the camera
and shadow passes.

## Block textures

Each 16 by 16 material layer has its own five-level mip chain. Mips use a
wrapped 4-tap binomial low-pass filter in linear light with alpha-premultiplied
color. Sampling across the repeat boundary reduces seams in source tiles whose
opposite edges differ. Alpha-tested layers preserve their base-level cutoff
coverage so distant foliage does not abruptly lose its silhouette.
Magnification keeps nearest filtering for the crisp block look; minification
and mip transitions filter across texels to reduce shimmer on distant and
oblique faces. Array layers are generated independently, so a mip cannot sample
a neighboring material layer.

## Current limits

- The shadow texel grid rotates with the sun, so individual shadow edges remain
  discretized at the map's resolution. Filtering softens the steps but cannot
  remove all temporal aliasing.
- Resident chunk coverage bounds which geometry can cast. Missing streamed
  chunks reduce the available shadow distance.
- The original 16 by 16 texture level remains unchanged for crisp close-up
  sampling. Source images with non-matching repeat borders can still show a
  seam at that level; wrapped filtering only reduces it in generated mips.
- Bias values are starting points for this renderer and its current depth
  format. Excessive bias causes detached or undersized shadows; insufficient
  bias can reveal self-shadowing.
