export function generateSeed(size: number): Uint8Array {
    let numbers: number[] = [];
    for (let i = 0; i < size; i++) {
        // generate a random number between 0 and 255
        let randomNumber = Math.floor(Math.random() * 255);
        numbers.push(randomNumber);
    }
    return new Uint8Array(numbers);
}
