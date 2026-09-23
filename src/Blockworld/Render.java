package Blockworld;

import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL20;

import java.io.IOException;
import java.nio.FloatBuffer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

public class Render {
	private static final float FOG_START = 260f;
	private static final float FOG_END = 430f;

	private World world;
	private int program;
	private int projectionLocation;
	private int viewLocation;
	private int modelLocation;
	private int lightPositionLocation;
	private int lightColorLocation;
	private int objectColorLocation;
	private int textureLocation;
	private int fogColorLocation;
	private int fogStartLocation;
	private int fogEndLocation;

	public void init(World world, Camera camera) throws IOException {
		this.world = world;
		int vertexShader = compileShader("src/Blockworld/vertex.glsl", GL20.GL_VERTEX_SHADER);
		int fragmentShader = 0;
		try {
			fragmentShader = compileShader("src/Blockworld/fragment.glsl", GL20.GL_FRAGMENT_SHADER);
			program = GL20.glCreateProgram();
			GL20.glAttachShader(program, vertexShader);
			GL20.glAttachShader(program, fragmentShader);
			GL20.glBindAttribLocation(program, Chunk.POSITION_ATTRIBUTE, "in_Position");
			GL20.glBindAttribLocation(program, Chunk.NORMAL_ATTRIBUTE, "in_Normal");
			GL20.glBindAttribLocation(program, Chunk.TEXCOORD_ATTRIBUTE, "in_TexCoord");
			GL20.glLinkProgram(program);
			if (GL20.glGetProgrami(program, GL20.GL_LINK_STATUS) == GL11.GL_FALSE) {
				throw new IllegalStateException("Could not link terrain shader: " + GL20.glGetProgramInfoLog(program));
			}
			projectionLocation = GL20.glGetUniformLocation(program, "projectionMatrix");
			viewLocation = GL20.glGetUniformLocation(program, "viewMatrix");
			modelLocation = GL20.glGetUniformLocation(program, "modelMatrix");
			lightPositionLocation = GL20.glGetUniformLocation(program, "lightPos");
			lightColorLocation = GL20.glGetUniformLocation(program, "lightColor");
			objectColorLocation = GL20.glGetUniformLocation(program, "objectColor");
			textureLocation = GL20.glGetUniformLocation(program, "textureAtlas");
			fogColorLocation = GL20.glGetUniformLocation(program, "fogColor");
			fogStartLocation = GL20.glGetUniformLocation(program, "fogStart");
			fogEndLocation = GL20.glGetUniformLocation(program, "fogEnd");
		} catch (RuntimeException e) {
			if (program != 0) GL20.glDeleteProgram(program);
			program = 0;
			throw e;
		} finally {
			GL20.glDeleteShader(vertexShader);
			if (fragmentShader != 0) GL20.glDeleteShader(fragmentShader);
		}
	}

	private int compileShader(String filename, int type) throws IOException {
		String source = new String(Files.readAllBytes(Paths.get(filename)), StandardCharsets.UTF_8);
		int shader = GL20.glCreateShader(type);
		GL20.glShaderSource(shader, source);
		GL20.glCompileShader(shader);
		if (GL20.glGetShaderi(shader, GL20.GL_COMPILE_STATUS) == GL11.GL_FALSE) {
			String log = GL20.glGetShaderInfoLog(shader);
			GL20.glDeleteShader(shader);
			throw new IllegalStateException("Could not compile " + filename + ": " + log);
		}
		return shader;
	}

	public void blocks() {
		FloatBuffer projection = BufferUtils.createFloatBuffer(16);
		FloatBuffer view = BufferUtils.createFloatBuffer(16);
		FloatBuffer model = BufferUtils.createFloatBuffer(16);
		GL11.glGetFloatv(GL11.GL_PROJECTION_MATRIX, projection);
		GL11.glGetFloatv(GL11.GL_MODELVIEW_MATRIX, view);
		model.put(new float[] {
			1f, 0f, 0f, 0f,
			0f, 1f, 0f, 0f,
			0f, 0f, 1f, 0f,
			0f, 0f, 0f, 1f
		}).flip();

		GL20.glUseProgram(program);
		GL20.glUniformMatrix4fv(projectionLocation, false, projection);
		GL20.glUniformMatrix4fv(viewLocation, false, view);
		GL20.glUniformMatrix4fv(modelLocation, false, model);
		GL20.glUniform3f(lightPositionLocation, 0f, 200f, 0f);
		GL20.glUniform3f(lightColorLocation, 1f, 1f, 1f);
		GL20.glUniform3f(objectColorLocation, 1f, 1f, 1f);
		GL20.glUniform1i(textureLocation, 0);
		GL20.glUniform3f(fogColorLocation, .59f, .78f, .91f);
		GL20.glUniform1f(fogStartLocation, FOG_START);
		GL20.glUniform1f(fogEndLocation, FOG_END);
		GL11.glColor3f(1f, 1f, 1f);
		TextureManager.bind();
		world.drawChunks();
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, 0);
		GL20.glUseProgram(0);
	}

	public void dispose() {
		if (program != 0) GL20.glDeleteProgram(program);
		program = 0;
	}
}
