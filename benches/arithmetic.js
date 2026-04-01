const addStage = (startVal) => startVal + 100;
const mulStage = (startVal) => startVal * 2;

const run = (startVal) => {
    return mulStage(addStage(startVal));
};

for (let i = 0; i < 100000; i++) {
    run(10);
}
