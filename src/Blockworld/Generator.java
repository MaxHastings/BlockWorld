package Blockworld;

import org.j3d.texture.procedural.PerlinNoiseGenerator;

public class Generator {

	public int heightAt(int x, int z, PerlinNoiseGenerator gen) {
		float broad = gen.noise2(x / 256f, z / 256f);
		float medium = gen.noise2(x / 64f, z / 64f) / 10f;
		float small = gen.noise2(x / 16f, z / 16f) / 30f;
		float fine = gen.noise2(x / 4f, z / 4f) / 60f;

		float detail;
		if (broad > .25f) detail = small + fine * 2f;
		else if (broad > 0f) detail = small + fine;
		else if (broad > -.25f) detail = (small + fine) / 2f;
		else detail = (small + fine) / 4f;

		return Math.round((broad + medium + detail) * 128f);
	}
}
