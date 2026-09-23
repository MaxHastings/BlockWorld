package Blockworld;

public class Collision {
	static final float PLAYER_RADIUS = .28f;
	static final float PLAYER_EYE_HEIGHT = 1.7f;
	static final float MAX_STEP_HEIGHT = 1f;

	private final World world;

	public Collision(World world) {
		this.world = world;
	}

	public int[] getVoxelCoords(Player player) {
		int x = Math.round(-player.x / Block.width);
		int y = Math.round(-player.y / Block.height);
		int z = Math.round(-player.z / Block.depth);
		return new int[] { x, y, z };
	}

	public float groundHeight(float worldX, float worldZ) {
		return world.terrainHeightAt(worldX, worldZ);
	}

	public boolean canOccupy(float worldX, float worldZ, float feetY) {
		float r = PLAYER_RADIUS;
		float highestGround = groundHeight(worldX, worldZ);
		highestGround = Math.max(highestGround, groundHeight(worldX + r, worldZ));
		highestGround = Math.max(highestGround, groundHeight(worldX - r, worldZ));
		highestGround = Math.max(highestGround, groundHeight(worldX, worldZ + r));
		highestGround = Math.max(highestGround, groundHeight(worldX, worldZ - r));
		highestGround = Math.max(highestGround, groundHeight(worldX + r, worldZ + r));
		highestGround = Math.max(highestGround, groundHeight(worldX + r, worldZ - r));
		highestGround = Math.max(highestGround, groundHeight(worldX - r, worldZ + r));
		highestGround = Math.max(highestGround, groundHeight(worldX - r, worldZ - r));
		return highestGround <= feetY + MAX_STEP_HEIGHT;
	}
}
