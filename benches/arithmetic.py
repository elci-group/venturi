def add_stage(start_val):
    return start_val + 100

def mul_stage(start_val):
    return start_val * 2

def run(start_val):
    return mul_stage(add_stage(start_val))

if __name__ == "__main__":
    for i in range(100000):
        run(10)
