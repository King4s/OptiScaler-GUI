import customtkinter as ctk

class App(ctk.CTk):
    def __init__(self):
        super().__init__()
        self.title("OptiScaler GUI")
        self.geometry("1024x768")

        self.label = ctk.CTkLabel(self, text="OptiScaler GUI")
        self.label.pack(pady=20)

if __name__ == "__main__":
    app = App()
    app.mainloop()