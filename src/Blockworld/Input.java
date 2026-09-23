package Blockworld;

import java.util.ArrayList;
import java.util.List;

import static org.lwjgl.glfw.GLFW.*;

public class Input {
	private final boolean[] keys = new boolean[GLFW_KEY_LAST + 1];
	private final List<Listener> listeners = new ArrayList<>();
	private double lastMouseX;
	private double lastMouseY;
	private float mouseDX;
	private float mouseDY;
	private boolean firstMouseEvent = true;

	public Input(long window) {
		glfwSetInputMode(window, GLFW_CURSOR, GLFW_CURSOR_DISABLED);
		glfwSetKeyCallback(window, (handle, key, scanCode, action, mods) -> {
			if (key < 0 || key >= keys.length) return;
			keys[key] = action != GLFW_RELEASE;
			if (action == GLFW_PRESS) {
				for (Listener listener : listeners) listener.keyPressed(key);
			}
		});
		glfwSetCursorPosCallback(window, (handle, x, y) -> {
			if (!firstMouseEvent) {
				mouseDX += (float)(x - lastMouseX);
				mouseDY += (float)(lastMouseY - y);
			}
			lastMouseX = x;
			lastMouseY = y;
			firstMouseEvent = false;
		});
	}

	public void poll() {
		for (Listener listener : listeners) {
			listener.mouseMoved(mouseDX, mouseDY);
			listener.remapKeys(keys);
		}
		mouseDX = 0;
		mouseDY = 0;
	}

	public void addListener(Listener listener) {
		listeners.add(listener);
	}

	interface Listener {
		void keyPressed(int key);
		void remapKeys(boolean[] keys);
		void mouseMoved(float mouseDX, float mouseDY);
	}
}
