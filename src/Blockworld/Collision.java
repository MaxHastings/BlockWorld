package Blockworld;

public class Collision {

	private World world;

	public Collision(World world){
		this.world = world;
	}

//	public Block getVoxelSpace(Player player){
//		int x = Math.round(-player.x / Block.width);
//		int y = Math.round(-player.y / Block.height);
//		int z = Math.round(-player.z / Block.depth);
//		Block[][][] blocks = world.blocks;
//
//		if(x < blocks.length && x >= 0 && y < blocks[0].length && y > 0 && z < blocks[0][0].length && z > 0){
//			//System.out.println("X: " + x + " Y: " + y + " Z: " + z);
//			return blocks[x][y][z];
//		}else{
//			return null;
//		}
//	}

	public int[] getVoxelCoords(Player player){
		int x = Math.round(-player.x / Block.width);
		int y = Math.round(-player.y / Block.height);
		int z = Math.round(-player.z / Block.depth);
		return new int[] {x,y,z};
	}

}
