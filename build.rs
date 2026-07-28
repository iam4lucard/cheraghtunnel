fn main() {
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=static/app.js");
    println!("cargo:rerun-if-changed=static/index.html");
    println!("cargo:rerun-if-changed=static/style.css");
}
