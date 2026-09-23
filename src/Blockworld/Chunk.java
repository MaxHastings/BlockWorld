package Blockworld;

import java.nio.FloatBuffer;

import org.j3d.texture.procedural.PerlinNoiseGenerator;
import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL15;
import org.lwjgl.opengl.GL20;

public class Chunk {

	public static final int sizeX = 64;
	public static final int sizeZ = 64;
	static final int POSITION_ATTRIBUTE = 0;
	static final int NORMAL_ATTRIBUTE = 1;
	static final int TEXCOORD_ATTRIBUTE = 2;

	private static final float[] TOP_UV = { 0f, 0f, .5f, 0f, .5f, .5f, 0f, .5f };
	private static final float[] SIDE_UV = { .5f, 0f, .5f, .5f, 1f, .5f, 1f, 0f };

	public final int chunkX;
	public final int chunkZ;
	private int vertexBuffer;
	private int vertexCount;

	public Chunk(int chunkX, int chunkZ, Generator generator, PerlinNoiseGenerator noise) {
		this.chunkX = chunkX;
		this.chunkZ = chunkZ;
		build(generator, noise);
	}

	private void build(Generator generator, PerlinNoiseGenerator noise) {
		int originX = chunkX * sizeX;
		int originZ = chunkZ * sizeZ;
		int[][] heights = new int[sizeX + 2][sizeZ + 2];
		for (int x = -1; x <= sizeX; x++) {
			for (int z = -1; z <= sizeZ; z++) {
				heights[x + 1][z + 1] = generator.heightAt(originX + x, originZ + z, noise);
			}
		}

		FloatList vertices = new FloatList(sizeX * sizeZ * 32);
		for (int x = 0; x < sizeX; x++) {
			for (int z = 0; z < sizeZ; z++) {
				float worldX = originX + x;
				float worldZ = originZ + z;
				float top = heights[x + 1][z + 1] + .5f;
				float east = heights[x + 2][z + 1] + .5f;
				float west = heights[x][z + 1] + .5f;
				float south = heights[x + 1][z + 2] + .5f;
				float north = heights[x + 1][z] + .5f;

				quad(vertices, 0f, 1f, 0f,
					worldX + .5f, top, worldZ + .5f,
					worldX - .5f, top, worldZ + .5f,
					worldX - .5f, top, worldZ - .5f,
					worldX + .5f, top, worldZ - .5f, TOP_UV);

				if (top > east) quad(vertices, 1f, 0f, 0f,
					worldX + .5f, top, worldZ + .5f,
					worldX + .5f, top, worldZ - .5f,
					worldX + .5f, east, worldZ - .5f,
					worldX + .5f, east, worldZ + .5f, SIDE_UV);
				if (top > west) quad(vertices, -1f, 0f, 0f,
					worldX - .5f, west, worldZ + .5f,
					worldX - .5f, west, worldZ - .5f,
					worldX - .5f, top, worldZ - .5f,
					worldX - .5f, top, worldZ + .5f, SIDE_UV);
				if (top > south) quad(vertices, 0f, 0f, 1f,
					worldX + .5f, south, worldZ + .5f,
					worldX - .5f, south, worldZ + .5f,
					worldX - .5f, top, worldZ + .5f,
					worldX + .5f, top, worldZ + .5f, SIDE_UV);
				if (top > north) quad(vertices, 0f, 0f, -1f,
					worldX + .5f, top, worldZ - .5f,
					worldX - .5f, top, worldZ - .5f,
					worldX - .5f, north, worldZ - .5f,
					worldX + .5f, north, worldZ - .5f, SIDE_UV);
			}
		}

		FloatBuffer buffer = BufferUtils.createFloatBuffer(vertices.size);
		buffer.put(vertices.values, 0, vertices.size);
		buffer.flip();
		vertexBuffer = GL15.glGenBuffers();
		GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, vertexBuffer);
		GL15.glBufferData(GL15.GL_ARRAY_BUFFER, buffer, GL15.GL_STATIC_DRAW);
		GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, 0);
		vertexCount = vertices.size / 8;
	}

	private static void quad(FloatList list,
		float nx, float ny, float nz,
		float ax, float ay, float az, float bx, float by, float bz,
		float cx, float cy, float cz, float dx, float dy, float dz,
		float[] uv) {
		list.add(ax, ay, az, nx, ny, nz, uv[0], uv[1]);
		list.add(bx, by, bz, nx, ny, nz, uv[2], uv[3]);
		list.add(cx, cy, cz, nx, ny, nz, uv[4], uv[5]);
		list.add(dx, dy, dz, nx, ny, nz, uv[6], uv[7]);
	}

	public void draw() {
		if (vertexBuffer == 0 || vertexCount == 0) return;
		GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, vertexBuffer);
		GL20.glVertexAttribPointer(POSITION_ATTRIBUTE, 3, GL11.GL_FLOAT, false, 32, 0L);
		GL20.glVertexAttribPointer(NORMAL_ATTRIBUTE, 3, GL11.GL_FLOAT, false, 32, 12L);
		GL20.glVertexAttribPointer(TEXCOORD_ATTRIBUTE, 2, GL11.GL_FLOAT, false, 32, 24L);
		GL20.glEnableVertexAttribArray(POSITION_ATTRIBUTE);
		GL20.glEnableVertexAttribArray(NORMAL_ATTRIBUTE);
		GL20.glEnableVertexAttribArray(TEXCOORD_ATTRIBUTE);
		GL11.glDrawArrays(GL11.GL_QUADS, 0, vertexCount);
		GL20.glDisableVertexAttribArray(TEXCOORD_ATTRIBUTE);
		GL20.glDisableVertexAttribArray(NORMAL_ATTRIBUTE);
		GL20.glDisableVertexAttribArray(POSITION_ATTRIBUTE);
		GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, 0);
	}

	public void dispose() {
		if (vertexBuffer != 0) GL15.glDeleteBuffers(vertexBuffer);
		vertexBuffer = 0;
		vertexCount = 0;
	}

	private static class FloatList {
		float[] values;
		int size;

		FloatList(int capacity) { values = new float[capacity]; }

		void add(float x, float y, float z, float nx, float ny, float nz, float u, float v) {
			if (size + 8 > values.length) {
				float[] larger = new float[Math.max(1024, values.length * 2)];
				System.arraycopy(values, 0, larger, 0, size);
				values = larger;
			}
			values[size++] = x;
			values[size++] = y;
			values[size++] = z;
			values[size++] = nx;
			values[size++] = ny;
			values[size++] = nz;
			values[size++] = u;
			values[size++] = v;
		}
	}
}
