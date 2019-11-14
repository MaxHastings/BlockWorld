package Blockworld;

import org.lwjgl.LWJGLException;
import org.lwjgl.Sys;
import org.lwjgl.input.Mouse;
import org.lwjgl.opengl.Display;
import org.lwjgl.opengl.DisplayMode;
import org.lwjgl.opengl.GL11;
import org.newdawn.slick.TrueTypeFont;

import java.awt.*;

import static org.lwjgl.util.glu.GLU.gluPerspective;

public class Screen {

	long lastFrame, lastFPS;
	
	int fps;
	
	World world;
	
	Camera camera;
	
	Input input;

	int width = 2560;
	int height = 1080;
	
	TrueTypeFont font;
	
	public static void main(String[] args) {
		Screen screen = new Screen();
		screen.start();
	}
	
	public void start(){
		try {
			DisplayMode displayMode = null;
			DisplayMode[] modes = Display.getAvailableDisplayModes();
			for(int i = 0; i < modes.length; i++){
				if(modes[i].getWidth() == width && modes[i].getHeight() == height && modes[i].isFullscreenCapable()){
					displayMode = modes[i];
				}
			}
			Display.setDisplayMode(displayMode);
			Display.setFullscreen(true);
			Display.create();
		} catch (LWJGLException e) {
			e.printStackTrace();
			System.exit(0);
		}
		
		Font awtFont = new Font("Impact", Font.PLAIN, 18);
		font = new TrueTypeFont(awtFont, false);
		
		GL11.glClearColor(.1f, .3f, .6f, 0);
		GL11.glViewport(0, 0, width, height);
		
		Mouse.setGrabbed(true);
	    
	    GL11.glEnable(GL11.GL_DEPTH_TEST);
	    
	    GL11.glEnable(GL11.GL_BLEND);
	    GL11.glBlendFunc(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA);
	    
		getDelta();
		lastFPS = getTime();
		
		input = new Input();
		world = new World();
		world.init(this);
	    
		while (!Display.isCloseRequested())
		{
			int delta = getDelta();
			update(delta);
			draw();
			
			input.poll();
			Display.update();
			Display.sync(144);
		}
		Display.destroy();
	}
	
	public void update(int delta){
		world.update(delta);
	}
	
	public void draw(){
		GL11.glClear(GL11.GL_DEPTH_BUFFER_BIT|GL11.GL_COLOR_BUFFER_BIT);
	    
		world.draw();
		
		make2D();
		int[] coords = world.collision.getVoxelCoords(world.player);
		font.drawString(10, 10, "FPS: " + realFPS);
		font.drawString(10, 30, "x: " + world.player.x);
		font.drawString(10, 50, "y: " + world.player.y);
		font.drawString(10, 70, "z: " + world.player.z);
		font.drawString(150, 30, "gridX: " + coords[0]);
		font.drawString(150, 50, "gridY: " + coords[1]);
		font.drawString(150, 70, "gridZ: " + coords[2]);
		font.drawString(10, 90, "rotationX: " + world.camera.rotationX);
		font.drawString(10, 110, "rotationY: " + world.camera.rotationY);
		make3D();
        
		updateFPS();
	}
	
	protected void make2D()
	{
		
		GL11.glMatrixMode(GL11.GL_PROJECTION);
	    GL11.glLoadIdentity();
	    GL11.glOrtho(0.0f, Display.getWidth(), Display.getHeight(), 0.0f, 0.0f, 1.0f);

	    GL11.glMatrixMode(GL11.GL_MODELVIEW);
	    GL11.glLoadIdentity();

	}

	protected void make3D()
	{
		GL11.glMatrixMode(GL11.GL_PROJECTION);
	    GL11.glLoadIdentity();

	    gluPerspective(70, (float)width/(float)height, 0.01f, 3000);
	    GL11.glMatrixMode(GL11.GL_MODELVIEW);

	}
	
	public long getTime(){
		return (Sys.getTime() * 1000) / Sys.getTimerResolution();
	}
	
	public int getDelta(){
		long time = getTime();
		int delta = (int) (time - lastFrame);
		lastFrame = time;
		
		return delta;
	}
	
	int realFPS = 0;
	
	public void updateFPS(){
		if(getTime() - lastFPS > 1000){
			Display.setTitle("FPS: " + fps);
			realFPS = fps;
			fps = 0;
			lastFPS += 1000;
		}
		fps++;
	}
}
