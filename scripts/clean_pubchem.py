import pandas as pd

print("🧪 Iniciando purificación del dataset de PubChem...")

# 1. Cargar el archivo caótico (Pandas maneja las comillas y comas perfectamente)
file_path = "PubChem_compound_FORMULA__C15H32.csv"
df = pd.read_csv(file_path)

if "SMILES" in df.columns:
    # 2. Extraer la columna y eliminar valores nulos
    smiles_list = df["SMILES"].dropna().tolist()

    structural_smiles = set()

    # 3. Limpiar estereoquímica 3D para quedarnos con grafos puramente topológicos
    for s in smiles_list:
        clean_s = s.replace("@", "").replace("/", "").replace("\\", "")
        structural_smiles.add(clean_s)

    structural_smiles = list(structural_smiles)

    # 4. Guardar el archivo limpio que espera nuestro Rust BenchmarkRunner
    out_df = pd.DataFrame({"smiles": structural_smiles})
    out_df.to_csv("data/chemistry/pentadecane_isomers.csv", index=False)

    print(f"✅ Éxito. Filas originales: {len(df)}")
    print(f"✅ Isómeros Topológicos Únicos Extraídos: {len(structural_smiles)}")
    print("El archivo 'data/chemistry/pentadecane_isomers.csv' está listo para Rust.")
else:
    print("❌ Error: No se encontró la columna SMILES.")
