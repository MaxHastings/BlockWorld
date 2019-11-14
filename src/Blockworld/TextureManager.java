package Blockworld;

import org.lwjgl.BufferUtils;
import org.newdawn.slick.opengl.Texture;
import org.newdawn.slick.opengl.TextureLoader;
import org.newdawn.slick.util.ResourceLoader;

import java.io.IOException;
import java.nio.ByteBuffer;

public class TextureManager {
	
	static Texture textures;
	
	static ByteBuffer buffer;
	
	public static void init(){
		try {
			textures = TextureLoader.getTexture("PNG", ResourceLoader.getResourceAsStream("res/textures.png"));
			buffer = BufferUtils.createByteBuffer(4 * textures.getTextureWidth() * textures.getTextureHeight());
			buffer.put(textures.getTextureData());
			buffer.flip();
		} catch (IOException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
	}
}