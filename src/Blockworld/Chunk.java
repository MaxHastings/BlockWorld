package Blockworld;

import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL15;

import java.nio.FloatBuffer;
import java.util.ArrayList;
import java.util.List;

public class Chunk {

	public static int sizeX = 512;
	
	public static int sizeZ = 512;

	static ArrayList<Integer> faceCountList = new ArrayList<>();

	static ArrayList<Integer> vboList = new ArrayList<>();
	
	static ArrayList<Integer> vbotList = new ArrayList<>();

	public static void sendVBO(Block[][] blocks, float offsetX, float offsetZ){
		
		List<Float> vertices = new ArrayList<Float>();
		
		List<Float> textureList = new ArrayList<Float>();
		
		float h = Block.height/2;
		float w = Block.width/2;
		float d = Block.depth/2;

		int faceCount = 0;

		for(int ix = 0; ix < blocks.length; ix++){
			for(int iz = 0; iz < blocks[ix].length; iz++) {
				Block block = blocks[ix][iz];
				float x = offsetX + (Block.width * ix);
				float z = offsetZ + (Block.depth * iz);
				float y1 = Block.height * block.getyCoord();
				textureList.add(0f);
				textureList.add(0f);
				textureList.add(0.5f);
				textureList.add(0f);
				textureList.add(0.5f);
				textureList.add(0.5f);
				textureList.add(0f);
				textureList.add(0.5f);

				vertices.add(x + w);
				vertices.add(y1 + h);
				vertices.add(z + d);
				vertices.add(x - w);
				vertices.add(y1 + h);
				vertices.add(z + d);
				vertices.add(x - w);
				vertices.add(y1 + h);
				vertices.add(z - d);
				vertices.add(x + w);
				vertices.add(y1 + h);
				vertices.add(z - d);
				faceCount += 4;

				if(ix < blocks.length - 1 ) {

					Block eBlock = blocks[ix + 1][iz];
					int ediff = eBlock.getyCoord() - block.getyCoord();

					for (int eiy = 0; eiy < Math.abs(ediff); eiy++) {

						float y;
						if (ediff > 0) {
							y = eBlock.getyCoord() - eiy;
						} else {
							y = block.getyCoord() - eiy;
						}

						textureList.add(0.5f);
						textureList.add(0f);
						textureList.add(0.5f);
						textureList.add(0.5f);
						textureList.add(1f);
						textureList.add(0.5f);
						textureList.add(1f);
						textureList.add(0f);

						vertices.add(x + w);
						vertices.add(y + h);
						vertices.add(z + d);
						vertices.add(x + w);
						vertices.add(y + h);
						vertices.add(z - d);
						vertices.add(x + w);
						vertices.add(y - h);
						vertices.add(z - d);
						vertices.add(x + w);
						vertices.add(y - h);
						vertices.add(z + d);
						faceCount += 4;
					}
				}

				if(iz < blocks[ix].length - 1) {
					Block sBlock = blocks[ix][iz + 1];
					int sdiff = sBlock.getyCoord() - block.getyCoord();

					for (int siy = 0; siy < Math.abs(sdiff); siy++) {

						float y;
						if (sdiff > 0) {
							y = sBlock.getyCoord() - siy;
						} else {
							y = block.getyCoord() - siy;
						}

						textureList.add(0.5f);
						textureList.add(0f);
						textureList.add(0.5f);
						textureList.add(0.5f);
						textureList.add(1f);
						textureList.add(0.5f);
						textureList.add(1f);
						textureList.add(0f);

						vertices.add(x - w);
						vertices.add(y - h);
						vertices.add(z + d);
						vertices.add(x + w);
						vertices.add(y - h);
						vertices.add(z + d);
						vertices.add(x + w);
						vertices.add(y + h);
						vertices.add(z + d);
						vertices.add(x - w);
						vertices.add(y + h);
						vertices.add(z + d);
						faceCount += 4;
					}
				}
			}
		}
		
		float[] verticesArray = new float[vertices.size()];
		for(int i = 0; i < verticesArray.length; i++){
			verticesArray[i] = vertices.get(i);
		}
		
		float[] textureArray = new float[textureList.size()];
		for(int i = 0; i < textureArray.length; i++){
			textureArray[i] = textureList.get(i);
		}
		
		System.out.println("Length: " + verticesArray.length + " FaceCount: " + faceCount);
		
        FloatBuffer textureBuffer = BufferUtils.createFloatBuffer(textureArray.length);
        textureBuffer.put(textureArray);
        textureBuffer.flip();

		int vbotId = GL15.glGenBuffers();
		vbotList.add(vbotId);
        GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, vbotId);
        GL15.glBufferData(GL15.GL_ARRAY_BUFFER, textureBuffer, GL15.GL_STATIC_DRAW);
        GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, 0);
		
        FloatBuffer verticesBuffer = BufferUtils.createFloatBuffer(verticesArray.length);
        verticesBuffer.put(verticesArray);
        verticesBuffer.flip();

        int vboId = GL15.glGenBuffers();
        vboList.add(vboId);
        GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, vboId);
        GL15.glBufferData(GL15.GL_ARRAY_BUFFER, verticesBuffer, GL15.GL_STATIC_DRAW);
        GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, 0);
        
        faceCountList.add(faceCount);
	}
	
}
