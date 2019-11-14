package Blockworld;

import java.nio.FloatBuffer;
import java.util.ArrayList;
import java.util.List;

import org.j3d.texture.procedural.PerlinNoiseGenerator;
import org.lwjgl.BufferUtils;
import org.lwjgl.input.Keyboard;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL15;
import org.lwjgl.opengl.GL20;
import org.lwjgl.opengl.GL30;

public class World {

	Render render;

	Camera camera;

	Collision collision;

	Player player;

	int grid;

	Block[][] blocks = new Block[0][0];

	Generator generate;

	int delta;
	
	public void generateWorld(){
		PerlinNoiseGenerator gen = new PerlinNoiseGenerator();

		int columns = 8;
		int rows = 8;
		for(int i = 0 ; i < columns * rows; i++) {
			int offsetX = Chunk.sizeX * (i % columns);
			int offsetZ = Chunk.sizeZ * (i / rows);
			blocks = generate.newChunk(offsetX, offsetZ, gen);
			Chunk.sendVBO(this.blocks, offsetX, offsetZ);
		}
	}

	public void init(Screen screen) {

		grid = 1;

		camera = new Camera();
		render = new Render();
		generate = new Generator();
		generateWorld();
		collision = new Collision(this);
		player = new Player(0, -50, 0);

		TextureManager.init();
		screen.input.addListener(new Input.Listener() {

			@Override
			public void keyPressed(int key) {
				if (key == Keyboard.KEY_ESCAPE) {
					System.exit(0);
				}
			}

			@Override
			public void remapKeys(boolean[] keys) {

				float speed = player.speed;
				//Block block = collision.getVoxelSpace(player);
				float h = Block.height / 2 * 10;
				float w = Block.width / 2 * 10;
				float d = Block.depth / 2 * 10;
				//if (block == null	|| collision.getVoxelSpace(player).isActive() == false) {
					if (keys[Keyboard.KEY_LSHIFT]) {
						speed /= 25;
					}
					if (keys[Keyboard.KEY_W]) {
						player.x += -(speed * delta
								* Math.sin(Math.toRadians(player.rotationY)) * Math
								.cos(Math.toRadians(player.rotationX)));
						player.y -= -(speed * delta * Math.sin(Math
								.toRadians(player.rotationX)));
						player.z -= -(speed * delta
								* Math.cos(Math.toRadians(player.rotationY)) * Math
								.cos(Math.toRadians(player.rotationX)));
					}
					if (keys[Keyboard.KEY_S]) {
						player.x += (speed * delta
								* Math.sin(Math.toRadians(player.rotationY)) * Math
								.cos(Math.toRadians(player.rotationX)));
						player.y -= (speed * delta * Math.sin(Math
								.toRadians(player.rotationX)));
						player.z -= (speed * delta
								* Math.cos(Math.toRadians(player.rotationY)) * Math
								.cos(Math.toRadians(player.rotationX)));

					}
					if (keys[Keyboard.KEY_A]) {
						player.x += -(speed * delta * Math.sin(Math
								.toRadians(player.rotationY - 90)));
						player.z -= -(speed * delta * Math.cos(Math
								.toRadians(player.rotationY - 90)));
					}
					if (keys[Keyboard.KEY_D]) {
						player.x += -(speed * delta * Math.sin(Math
								.toRadians(player.rotationY + 90)));
						player.z -= -(speed * delta * Math.cos(Math
								.toRadians(player.rotationY + 90)));
					}
					if (keys[Keyboard.KEY_R]) {
						generateWorld();
					}
				//} else {
				//	player.y -= 0.01f;
				//}

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

	public void update(int delta) {

		this.delta = delta;

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
