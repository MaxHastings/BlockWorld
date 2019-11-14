package Blockworld;

import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL15;
import org.lwjgl.opengl.GL20;
import org.lwjgl.opengl.GL30;

import java.io.File;

import static org.lwjgl.opengl.GL11.*;
import static org.lwjgl.opengl.GL12.GL_TEXTURE_MAX_LEVEL;
import static org.lwjgl.opengl.GL15.GL_ARRAY_BUFFER;
import static org.lwjgl.opengl.GL15.glBindBuffer;

public class Render {

	Camera camera;
	
	World world;

	private int vsId;

	private int fsId;

	private int pId;

	public void init(World world, Camera camera){
		this.camera = camera;
		this.world = world;
		// Load the vertex shader
		vsId = Util.loadShader("./src/Blockworld/vertex.glsl", GL20.GL_VERTEX_SHADER);
		// Load the fragment shader
		fsId = Util.loadShader("./src/Blockworld/fragment.glsl", GL20.GL_FRAGMENT_SHADER);

		// Create a new shader program that links both shaders
		pId = GL20.glCreateProgram();
		GL20.glAttachShader(pId, vsId);
		GL20.glAttachShader(pId, fsId);

		GL20.glLinkProgram(pId);
		GL20.glValidateProgram(pId);
	}
	
	public void blocks(){
		
		glColor3f ( 1.0f, 1.0f, 1.0f ) ;
    	GL11.glClear(GL11.GL_COLOR_BUFFER_BIT | GL11.GL_DEPTH_BUFFER_BIT);

		//GL20.glUseProgram(pId);

		TextureManager.textures.bind();
		glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, TextureManager.textures.getImageWidth(), TextureManager.textures.getImageHeight(), 0, GL_RGBA, GL_UNSIGNED_BYTE, TextureManager.buffer);
		GL30.glGenerateMipmap(GL_TEXTURE_2D);

		glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST_MIPMAP_LINEAR);
		glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
		glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAX_LEVEL, 4);

		for(int i = 0; i < Chunk.vboList.size() && i < Chunk.vbotList.size(); i++) {
			glBindBuffer(GL_ARRAY_BUFFER, Chunk.vbotList.get(i));
			glTexCoordPointer(2, GL_FLOAT, 0, 0);

			// Bind to the index VBO that has all the information about the order of the vertices
			GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, Chunk.vboList.get(i));
			glVertexPointer(3, GL_FLOAT, 0, 0);


			glEnableClientState(GL_TEXTURE_COORD_ARRAY);
			glEnableClientState(GL_VERTEX_ARRAY);
			glDrawArrays(GL_QUADS, 0, Chunk.faceCountList.get(i));
			glDisableClientState(GL_VERTEX_ARRAY);
			glDisableClientState(GL_TEXTURE_COORD_ARRAY);

		}
         
        // Put everything back to default (deselect)
        GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, 0);
		glBindTexture(GL_TEXTURE_2D, 0);
		GL20.glUseProgram(0);
	}
	
}
