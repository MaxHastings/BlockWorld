package Blockworld;

import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import java.io.IOException;

import org.j3d.texture.procedural.PerlinNoiseGenerator;
import org.lwjgl.opengl.GL11;

import static org.lwjgl.glfw.GLFW.*;

public class World {

	private static final int VIEW_RADIUS = 6;

	Render render;

	Camera camera;

	Collision collision;

	Player player;

	Generator generate;
	final Map<Long, Chunk> chunks = new HashMap<>();
	private PerlinNoiseGenerator terrainNoise;
	private boolean regenerateDown;
	private boolean walkMode;
	private boolean grounded;
	private float verticalVelocity;
	private static final float GRAVITY = 24f;
	private static final float JUMP_VELOCITY = 8.5f;
	private static final float WALK_SPEED = .004f;

	int delta;
	
	public void generateWorld() {
		clearChunks();
		terrainNoise = new PerlinNoiseGenerator();
		int spawnHeight = generate.heightAt(0, 0, terrainNoise);
		player = new Player(0, -(spawnHeight + 8), 0);
	}

	private boolean loadNextChunk() {
		int worldX = (int)Math.floor(-player.x);
		int worldZ = (int)Math.floor(-player.z);
		int centerX = Math.floorDiv(worldX, Chunk.sizeX);
		int centerZ = Math.floorDiv(worldZ, Chunk.sizeZ);
		int bestX = 0, bestZ = 0;
		int bestDistance = Integer.MAX_VALUE;
		boolean found = false;

		for (int dx = -VIEW_RADIUS; dx <= VIEW_RADIUS; dx++) {
			for (int dz = -VIEW_RADIUS; dz <= VIEW_RADIUS; dz++) {
				int distance = dx * dx + dz * dz;
				if (distance > VIEW_RADIUS * VIEW_RADIUS) continue;
				int chunkX = centerX + dx;
				int chunkZ = centerZ + dz;
				if (chunks.containsKey(chunkKey(chunkX, chunkZ))) continue;
				if (distance < bestDistance) {
					bestDistance = distance;
					bestX = chunkX;
					bestZ = chunkZ;
					found = true;
				}
			}
		}

		if (!found) return false;
		Chunk chunk = new Chunk(bestX, bestZ, generate, terrainNoise);
		chunks.put(chunkKey(bestX, bestZ), chunk);
		return true;
	}

	private void removeDistantChunks() {
		int centerX = Math.floorDiv((int)Math.floor(-player.x), Chunk.sizeX);
		int centerZ = Math.floorDiv((int)Math.floor(-player.z), Chunk.sizeZ);
		int unloadRadius = VIEW_RADIUS + 1;
		Iterator<Map.Entry<Long, Chunk>> iterator = chunks.entrySet().iterator();
		while (iterator.hasNext()) {
			Chunk chunk = iterator.next().getValue();
			int dx = chunk.chunkX - centerX;
			int dz = chunk.chunkZ - centerZ;
			if (dx * dx + dz * dz > unloadRadius * unloadRadius) {
				chunk.dispose();
				iterator.remove();
			}
		}
	}

	private long chunkKey(int x, int z) {
		return ((long)x << 32) | (z & 0xffffffffL);
	}

	private void clearChunks() {
		for (Chunk chunk : chunks.values()) chunk.dispose();
		chunks.clear();
	}

	public void drawChunks() {
		for (Chunk chunk : chunks.values()) chunk.draw();
	}

	public void dispose() {
		clearChunks();
		if (render != null) render.dispose();
	}

	public void init(Screen screen) throws IOException {

		camera = new Camera();
		render = new Render();
		generate = new Generator();
		generateWorld();
		collision = new Collision(this);

		TextureManager.init();
		screen.input.addListener(new Input.Listener() {

			@Override
			public void keyPressed(int key) {
				if (key == GLFW_KEY_ESCAPE) {
					screen.requestClose();
				} else if (key == GLFW_KEY_G) {
					walkMode = !walkMode;
					verticalVelocity = 0f;
					grounded = false;
				} else if (key == GLFW_KEY_SPACE && walkMode && grounded) {
					verticalVelocity = JUMP_VELOCITY;
					grounded = false;
				}
			}

			@Override
			public void remapKeys(boolean[] keys) {
				if (walkMode) moveOnGround(keys, WALK_SPEED * delta);
				else {
					float speed = player.speed * (keys[GLFW_KEY_LEFT_SHIFT] ? 10f : 1f);
					moveInFlight(keys, speed * delta);
				}
				if (keys[GLFW_KEY_R] && !regenerateDown) generateWorld();
				regenerateDown = keys[GLFW_KEY_R];
			}

			@Override
			public void mouseMoved(float mouseDX, float mouseDY) {
				float rotationY = player.rotationY;
				float rotationX = player.rotationX;
				if (rotationY + mouseDX >= 360) {
					rotationY = rotationY + mouseDX - 360;
				} else if (rotationY + mouseDX < 0) {
					rotationY = 360 - rotationY + mouseDX;
				} else {
					rotationY += mouseDX;
				}
				if (rotationX - mouseDY >= -89 && rotationX - mouseDY <= 89) {
					rotationX += -mouseDY;
				} else if (rotationX - mouseDY < -89) {
					rotationX = -89;
				} else if (rotationX - mouseDY > 89) {
					rotationX = 89;
				}
				player.rotationX = rotationX;
				player.rotationY = rotationY;

			}
		});

		render.init(this, camera);
	}

	private void moveInFlight(boolean[] keys, float distance) {
		float forward = (keys[GLFW_KEY_W] ? 1f : 0f) - (keys[GLFW_KEY_S] ? 1f : 0f);
		float strafe = (keys[GLFW_KEY_D] ? 1f : 0f) - (keys[GLFW_KEY_A] ? 1f : 0f);
		float yaw = (float)Math.toRadians(player.rotationY);
		float pitch = (float)Math.toRadians(player.rotationX);
		float worldX = -player.x;
		float worldY = -player.y;
		float worldZ = -player.z;
		worldX += (float)Math.sin(yaw) * (float)Math.cos(pitch) * forward * distance;
		worldX += (float)Math.cos(yaw) * strafe * distance;
		worldY -= (float)Math.sin(pitch) * forward * distance;
		worldZ -= (float)Math.cos(yaw) * (float)Math.cos(pitch) * forward * distance;
		worldZ += (float)Math.sin(yaw) * strafe * distance;
		player.x = -worldX;
		player.y = -worldY;
		player.z = -worldZ;
	}

	private void moveOnGround(boolean[] keys, float distance) {
		float forward = (keys[GLFW_KEY_W] ? 1f : 0f) - (keys[GLFW_KEY_S] ? 1f : 0f);
		float strafe = (keys[GLFW_KEY_D] ? 1f : 0f) - (keys[GLFW_KEY_A] ? 1f : 0f);
		if (forward == 0f && strafe == 0f) return;
		float length = (float)Math.sqrt(forward * forward + strafe * strafe);
		forward /= length;
		strafe /= length;

		float yaw = (float)Math.toRadians(player.rotationY);
		float dx = ((float)Math.sin(yaw) * forward + (float)Math.cos(yaw) * strafe) * distance;
		float dz = (-(float)Math.cos(yaw) * forward + (float)Math.sin(yaw) * strafe) * distance;
		float worldX = -player.x;
		float worldZ = -player.z;
		float feetY = -player.y - Collision.PLAYER_EYE_HEIGHT;

		if (collision.canOccupy(worldX + dx, worldZ + dz, feetY)) {
			worldX += dx;
			worldZ += dz;
		} else {
			if (collision.canOccupy(worldX + dx, worldZ, feetY)) worldX += dx;
			if (collision.canOccupy(worldX, worldZ + dz, feetY)) worldZ += dz;
		}
		player.x = -worldX;
		player.z = -worldZ;
	}

	public void update(int delta) {

		this.delta = Math.min(delta, 50);
		if (player != null && terrainNoise != null) {
			if (walkMode) updateGravity(this.delta / 1000f);
			removeDistantChunks();
			loadNextChunk();
		}

	}

	float terrainHeightAt(float worldX, float worldZ) {
		int x = (int)Math.floor(worldX + .5f);
		int z = (int)Math.floor(worldZ + .5f);
		return generate.heightAt(x, z, terrainNoise) + .5f;
	}

	private void updateGravity(float seconds) {
		float worldX = -player.x;
		float worldY = -player.y;
		float worldZ = -player.z;
		verticalVelocity -= GRAVITY * seconds;
		worldY += verticalVelocity * seconds;

		float floorY = collision.groundHeight(worldX, worldZ) + Collision.PLAYER_EYE_HEIGHT;
		if (worldY <= floorY) {
			worldY = floorY;
			verticalVelocity = 0f;
			grounded = true;
		} else {
			grounded = false;
		}
		player.y = -worldY;
	}

	boolean isWalkMode() {
		return walkMode;
	}

	public void draw() {

		camera.set(player.x, player.y, player.z, player.rotationX, player.rotationY);

		float camX = camera.x;
		float camY = camera.y;
		float camZ = camera.z;

		GL11.glRotatef(camera.rotationX, 1, 0, 0);
		GL11.glRotatef(camera.rotationY, 0, 1, 0);

		GL11.glTranslatef(camX, camY, camZ);

		render.blocks();

	}

}
