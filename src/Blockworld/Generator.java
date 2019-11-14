package Blockworld;

import java.util.Random;

import org.j3d.texture.procedural.PerlinNoiseGenerator;

public class Generator {
	
	int min, max = 0;

	public Block[][] newChunk(int offSetX, int offSetZ, PerlinNoiseGenerator gen) {
				
		int sizeX = Chunk.sizeX;
		int sizeZ = Chunk.sizeZ;

		Block[][] blocks = new Block[sizeX][sizeZ];
		for(int ix = 0; ix < blocks.length; ix++){
			for(int iz = 0; iz < blocks[ix].length; iz++){
				
				float noise2 = 0;

				int x = ix + offSetX;
				int z = iz + offSetZ;
				float bignoise = gen.noise2(x / 256f, z / 256f);
				float mediumnoise = gen.noise2(x / 64f, z / 64f) / 10;
				float smallnoise = gen.noise2(x / 16f, z / 16f) / 30;
				float vsmallnoise = gen.noise2(x / 4f, z / 4f) / 60;

				noise2 += bignoise;


				if(bignoise > 0.25){
					noise2 += smallnoise + vsmallnoise  * 2;
				}else if (bignoise > 0){
					noise2 += (smallnoise + vsmallnoise);
				}else if (bignoise > -0.25){
					noise2 += (smallnoise + vsmallnoise) / 2;
				}else{
					noise2 += (smallnoise + vsmallnoise) / 4;
				}

				noise2 += mediumnoise;
				//noise2 = Math.abs(noise2);
				noise2 *= 128;
				

				Block block = new Block();
				block.setyCoord(Math.round(noise2));
				blocks[ix][iz] = block;
			}
		}

		return blocks;
	}
	
	public static int randInt(int min, int max) {

	    // NOTE: Usually this should be a field rather than a method
	    // variable so that it is not re-seeded every call.
	    Random rand = new Random();

	    // nextInt is normally exclusive of the top value,
	    // so add 1 to make it inclusive
	    int randomNum = rand.nextInt((max - min) + 1) + min;

	    return randomNum;
	}

}
