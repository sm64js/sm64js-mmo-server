export const int16 = (num) => {
    num = num > 32767 ? num - 65536 : num;
    num = num < -32768 ? num + 65536 : num;
    return num;
}

export const sins = (num) => {
    return Math.sin(num / 0x8000 * Math.PI);
}

export const coss = (num) => {
    return Math.cos(num / 0x8000 * Math.PI);
}

export const lerp = (min,max,amt) => { 
	if (amt < 0) amt = 0;
	if (amt > 1) amt = 1;
    return ((min * (1 - amt)) + (max * amt))
}
