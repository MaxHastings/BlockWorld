package Blockworld;

import org.lwjgl.opengl.GL;
import org.lwjgl.opengl.GL11;

import java.io.IOException;

import static org.lwjgl.glfw.GLFW.*;

public class Screen {
	private long window;
	private long lastFrame;
	private long lastFPS;
	private int fps;
	private int width = 1280;
	private int height = 800;

	World world;
	Input input;

	public static void main(String[] args) {
		new Screen().start(args);
	}

	public void start(String[] args) {
		if (!glfwInit()) throw new IllegalStateException("Could not initialize GLFW");
		try {
			glfwDefaultWindowHints();
			// The terrain renderer uses OpenGL's compatibility profile.
			glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 2);
			glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 1);
			glfwWindowHint(GLFW_VISIBLE, GLFW_FALSE);
			glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);

			long monitor = 0;
			for (String arg : args) {
				if ("--fullscreen".equals(arg)) monitor = glfwGetPrimaryMonitor();
			}
			if (monitor != 0) {
				org.lwjgl.glfw.GLFWVidMode mode = glfwGetVideoMode(monitor);
				if (mode == null) throw new IllegalStateException("Could not read the display mode");
				width = mode.width();
				height = mode.height();
			}

			window = glfwCreateWindow(width, height, "BlockWorld", monitor, 0);
			if (window == 0) throw new IllegalStateException("Could not create the BlockWorld window");
			glfwMakeContextCurrent(window);
			GL.createCapabilities();
			glfwSetFramebufferSizeCallback(window, (handle, framebufferWidth, framebufferHeight) ->
				GL11.glViewport(0, 0, framebufferWidth, framebufferHeight));
			glfwSwapInterval(1);
			glfwShowWindow(window);

			input = new Input(window);
			world = new World();
			initializeOpenGL();
			try {
				world.init(this);
			} catch (IOException e) {
				throw new IllegalStateException("Could not load BlockWorld textures", e);
			}

			lastFrame = getTime();
			lastFPS = lastFrame;
			while (!glfwWindowShouldClose(window)) {
				glfwPollEvents();
				long now = getTime();
				int delta = (int)Math.min(now - lastFrame, 50);
				lastFrame = now;
				world.update(delta);
				input.poll();
				draw();
				glfwSwapBuffers(window);
				updateFPS();
			}
		} finally {
			if (world != null) world.dispose();
			TextureManager.dispose();
			if (window != 0) {
				org.lwjgl.glfw.Callbacks.glfwFreeCallbacks(window);
				glfwDestroyWindow(window);
			}
			glfwTerminate();
		}
	}

	private void initializeOpenGL() {
		float[] sky = { .59f, .78f, .91f, 1f };
		GL11.glClearColor(sky[0], sky[1], sky[2], sky[3]);
		int[] framebufferWidth = new int[1];
		int[] framebufferHeight = new int[1];
		glfwGetFramebufferSize(window, framebufferWidth, framebufferHeight);
		GL11.glViewport(0, 0, framebufferWidth[0], framebufferHeight[0]);
		GL11.glEnable(GL11.GL_DEPTH_TEST);
		GL11.glEnable(GL11.GL_TEXTURE_2D);
		GL11.glEnable(GL11.GL_BLEND);
		GL11.glBlendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
		make3D();
	}

	private void draw() {
		GL11.glClear(GL11.GL_DEPTH_BUFFER_BIT | GL11.GL_COLOR_BUFFER_BIT);
		GL11.glMatrixMode(GL11.GL_MODELVIEW);
		GL11.glLoadIdentity();
		world.draw();
	}

	private void make3D() {
		GL11.glMatrixMode(GL11.GL_PROJECTION);
		GL11.glLoadIdentity();
		double near = .01;
		double far = 3000;
		double top = Math.tan(Math.toRadians(70) / 2) * near;
		double right = top * width / height;
		GL11.glFrustum(-right, right, -top, top, near, far);
		GL11.glMatrixMode(GL11.GL_MODELVIEW);
	}

	private long getTime() {
		return System.nanoTime() / 1_000_000;
	}

	private void updateFPS() {
		fps++;
		if (getTime() - lastFPS >= 1000) {
			int[] pos = world.collision.getVoxelCoords(world.player);
			String mode = world.isWalkMode() ? "Walk (G to fly)" : "Flight (G to walk)";
			glfwSetWindowTitle(window, "BlockWorld | " + mode + " | " + fps + " FPS | " +
				pos[0] + ", " + pos[1] + ", " + pos[2]);
			fps = 0;
			lastFPS = getTime();
		}
	}

	void requestClose() {
		glfwSetWindowShouldClose(window, true);
	}
}
