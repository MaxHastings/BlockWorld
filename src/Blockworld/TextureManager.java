package Blockworld;

import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;

import javax.imageio.ImageIO;
import java.awt.image.BufferedImage;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;

public class TextureManager {
	private static int textureId;

	public static void init() throws IOException {
		InputStream stream = TextureManager.class.getResourceAsStream("/textures.png");
		if (stream == null) throw new IOException("textures.png was not found on the classpath");

		BufferedImage image;
		try (InputStream input = stream) {
			image = ImageIO.read(input);
		}
		if (image == null) throw new IOException("res/textures.png is not a supported image");

		int width = image.getWidth();
		int height = image.getHeight();
		ByteBuffer pixels = BufferUtils.createByteBuffer(width * height * 4);
		for (int y = 0; y < height; y++) {
			for (int x = 0; x < width; x++) {
				int argb = image.getRGB(x, y);
				pixels.put((byte) (argb >> 16));
				pixels.put((byte) (argb >> 8));
				pixels.put((byte) argb);
				pixels.put((byte) (argb >> 24));
			}
		}
		pixels.flip();

		textureId = GL11.glGenTextures();
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, textureId);
		GL11.glTexImage2D(GL11.GL_TEXTURE_2D, 0, GL11.GL_RGBA, width, height, 0,
			GL11.GL_RGBA, GL11.GL_UNSIGNED_BYTE, pixels);
		GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MIN_FILTER, GL11.GL_NEAREST);
		GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MAG_FILTER, GL11.GL_NEAREST);
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, 0);
	}

	public static void bind() {
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, textureId);
	}

	public static void dispose() {
		if (textureId != 0) GL11.glDeleteTextures(textureId);
		textureId = 0;
	}
}
