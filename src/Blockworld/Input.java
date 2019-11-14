package Blockworld;

import org.lwjgl.input.Keyboard;
import org.lwjgl.input.Mouse;

import java.util.ArrayList;
import java.util.List;

public class Input {
	
	boolean[] keys = new boolean[256];
	
	List<Input.Listener> listeners = new ArrayList<Input.Listener>();
	
	public void poll(){
		
		if(Mouse.isGrabbed()){
			
			
			
			for(int i = 0; i < listeners.size(); i++){
				Listener listener = listeners.get(i);
				listener.mouseMoved(Mouse.getDX(), Mouse.getDY());
			}
		}
		
		for(int i = 0; i < keys.length; i++){
			keys[i] = Keyboard.isKeyDown(i);
		}
		
		for(int i = 0; i < listeners.size(); i++){
			Listener listener = listeners.get(i);
			listener.remapKeys(keys);
		}
		
		while(Keyboard.next()){
			
			int key = Keyboard.getEventKey();
			
			for(int i = 0; i < listeners.size(); i++){
				Listener listener = listeners.get(i);
				listener.keyPressed(key);
			}
		}
		
		
	}
	
	public void addListener(Listener listener){
		listeners.add(listener);
	}
	
	interface Listener{
		
		public void keyPressed(int key);
		
		public void remapKeys(boolean[] keys);
		
		public void mouseMoved(float mouseDX, float mouseDY);
		
	}

}
