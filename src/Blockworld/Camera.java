package Blockworld;

public class Camera {

	float x,y,z = 0;
	
	float rotationX, rotationY, rotationZ = 0;
	
	public void set(float x, float y, float z, float rotationX, float rotationY){
		this.rotationX = rotationX;
		this.rotationY = rotationY;
		this.x = x;
		this.y = y;
		this.z = z;
	}

}
