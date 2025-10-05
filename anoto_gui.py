#!/usr/bin/env python3
"""
Anoto GUI - Computer vision test for Anoto PDF with Anoto dot paper
A simple GUI application to load and analyze Anoto dot paper patterns from PDF files.
"""

import tkinter as tk
from tkinter import filedialog, messagebox, ttk
import cv2
import numpy as np
from PIL import Image, ImageTk
import os


class AnotoGUI:
    """Main GUI application for Anoto dot paper analysis."""
    
    def __init__(self, root):
        self.root = root
        self.root.title("Anoto GUI - Dot Paper Analyzer")
        self.root.geometry("1000x700")
        
        self.current_image = None
        self.processed_image = None
        self.image_path = None
        
        self.setup_ui()
    
    def setup_ui(self):
        """Setup the user interface."""
        # Menu bar
        menubar = tk.Menu(self.root)
        self.root.config(menu=menubar)
        
        file_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="File", menu=file_menu)
        file_menu.add_command(label="Load Image", command=self.load_image)
        file_menu.add_command(label="Load PDF", command=self.load_pdf)
        file_menu.add_separator()
        file_menu.add_command(label="Exit", command=self.root.quit)
        
        # Top frame for controls
        control_frame = tk.Frame(self.root, relief=tk.RAISED, borderwidth=1)
        control_frame.pack(side=tk.TOP, fill=tk.X, padx=5, pady=5)
        
        tk.Button(control_frame, text="Load Image", command=self.load_image, 
                 bg="#4CAF50", fg="white", padx=10).pack(side=tk.LEFT, padx=5)
        tk.Button(control_frame, text="Load PDF", command=self.load_pdf,
                 bg="#2196F3", fg="white", padx=10).pack(side=tk.LEFT, padx=5)
        tk.Button(control_frame, text="Detect Dots", command=self.detect_dots,
                 bg="#FF9800", fg="white", padx=10).pack(side=tk.LEFT, padx=5)
        tk.Button(control_frame, text="Reset", command=self.reset_view,
                 bg="#f44336", fg="white", padx=10).pack(side=tk.LEFT, padx=5)
        
        # Parameters frame
        params_frame = tk.LabelFrame(self.root, text="Detection Parameters", 
                                     relief=tk.RIDGE, borderwidth=2)
        params_frame.pack(side=tk.TOP, fill=tk.X, padx=5, pady=5)
        
        # Threshold slider
        tk.Label(params_frame, text="Threshold:").pack(side=tk.LEFT, padx=5)
        self.threshold_var = tk.IntVar(value=127)
        self.threshold_slider = tk.Scale(params_frame, from_=0, to=255, 
                                        orient=tk.HORIZONTAL, variable=self.threshold_var,
                                        length=200)
        self.threshold_slider.pack(side=tk.LEFT, padx=5)
        
        # Min area slider
        tk.Label(params_frame, text="Min Dot Area:").pack(side=tk.LEFT, padx=5)
        self.min_area_var = tk.IntVar(value=5)
        self.min_area_slider = tk.Scale(params_frame, from_=1, to=50, 
                                       orient=tk.HORIZONTAL, variable=self.min_area_var,
                                       length=200)
        self.min_area_slider.pack(side=tk.LEFT, padx=5)
        
        # Info label
        self.info_label = tk.Label(self.root, text="Load an image or PDF to begin", 
                                   relief=tk.SUNKEN, anchor=tk.W)
        self.info_label.pack(side=tk.BOTTOM, fill=tk.X)
        
        # Main canvas frame
        canvas_frame = tk.Frame(self.root)
        canvas_frame.pack(side=tk.TOP, fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        # Original image canvas
        original_frame = tk.LabelFrame(canvas_frame, text="Original Image", 
                                       relief=tk.RIDGE, borderwidth=2)
        original_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=5)
        
        self.original_canvas = tk.Canvas(original_frame, bg='gray')
        self.original_canvas.pack(fill=tk.BOTH, expand=True)
        
        # Processed image canvas
        processed_frame = tk.LabelFrame(canvas_frame, text="Processed Image", 
                                        relief=tk.RIDGE, borderwidth=2)
        processed_frame.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True, padx=5)
        
        self.processed_canvas = tk.Canvas(processed_frame, bg='gray')
        self.processed_canvas.pack(fill=tk.BOTH, expand=True)
    
    def load_image(self):
        """Load an image file."""
        file_path = filedialog.askopenfilename(
            title="Select Image",
            filetypes=[("Image files", "*.png *.jpg *.jpeg *.bmp *.tiff"), 
                      ("All files", "*.*")]
        )
        
        if file_path:
            try:
                self.current_image = cv2.imread(file_path)
                if self.current_image is None:
                    messagebox.showerror("Error", "Failed to load image")
                    return
                
                self.image_path = file_path
                self.processed_image = None
                self.display_image(self.current_image, self.original_canvas)
                self.info_label.config(text=f"Loaded: {os.path.basename(file_path)}")
                
                # Clear processed canvas
                self.processed_canvas.delete("all")
                
            except Exception as e:
                messagebox.showerror("Error", f"Failed to load image: {str(e)}")
    
    def load_pdf(self):
        """Load a PDF file and convert first page to image."""
        file_path = filedialog.askopenfilename(
            title="Select PDF",
            filetypes=[("PDF files", "*.pdf"), ("All files", "*.*")]
        )
        
        if file_path:
            try:
                from pdf2image import convert_from_path
                
                # Convert first page to image
                images = convert_from_path(file_path, first_page=1, last_page=1, dpi=200)
                
                if images:
                    # Convert PIL image to OpenCV format
                    pil_image = images[0]
                    self.current_image = cv2.cvtColor(np.array(pil_image), cv2.COLOR_RGB2BGR)
                    self.image_path = file_path
                    self.processed_image = None
                    
                    self.display_image(self.current_image, self.original_canvas)
                    self.info_label.config(text=f"Loaded PDF: {os.path.basename(file_path)}")
                    
                    # Clear processed canvas
                    self.processed_canvas.delete("all")
                else:
                    messagebox.showerror("Error", "No pages found in PDF")
                    
            except ImportError:
                messagebox.showerror("Error", 
                    "pdf2image library not installed. Please install with: pip install pdf2image\n"
                    "Also requires poppler-utils system package.")
            except Exception as e:
                messagebox.showerror("Error", f"Failed to load PDF: {str(e)}")
    
    def detect_dots(self):
        """Detect dot patterns in the loaded image."""
        if self.current_image is None:
            messagebox.showwarning("Warning", "Please load an image first")
            return
        
        try:
            # Convert to grayscale
            gray = cv2.cvtColor(self.current_image, cv2.COLOR_BGR2GRAY)
            
            # Apply threshold
            threshold_value = self.threshold_var.get()
            _, binary = cv2.threshold(gray, threshold_value, 255, cv2.THRESH_BINARY_INV)
            
            # Find contours (dots)
            contours, _ = cv2.findContours(binary, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
            
            # Filter contours by area
            min_area = self.min_area_var.get()
            filtered_contours = [cnt for cnt in contours if cv2.contourArea(cnt) >= min_area]
            
            # Draw contours on a copy of the original image
            result = self.current_image.copy()
            cv2.drawContours(result, filtered_contours, -1, (0, 255, 0), 2)
            
            # Draw centroids
            for cnt in filtered_contours:
                M = cv2.moments(cnt)
                if M['m00'] != 0:
                    cx = int(M['m10'] / M['m00'])
                    cy = int(M['m01'] / M['m00'])
                    cv2.circle(result, (cx, cy), 3, (0, 0, 255), -1)
            
            self.processed_image = result
            self.display_image(result, self.processed_canvas)
            
            # Update info
            info_text = (f"Detected {len(filtered_contours)} dots | "
                        f"Threshold: {threshold_value} | Min Area: {min_area}")
            self.info_label.config(text=info_text)
            
        except Exception as e:
            messagebox.showerror("Error", f"Failed to detect dots: {str(e)}")
    
    def reset_view(self):
        """Reset the view to original image."""
        if self.current_image is not None:
            self.processed_image = None
            self.processed_canvas.delete("all")
            self.info_label.config(text=f"Reset view - {os.path.basename(self.image_path) if self.image_path else ''}")
    
    def display_image(self, cv_image, canvas):
        """Display OpenCV image on canvas."""
        # Convert BGR to RGB
        rgb_image = cv2.cvtColor(cv_image, cv2.COLOR_BGR2RGB)
        
        # Convert to PIL Image
        pil_image = Image.fromarray(rgb_image)
        
        # Get canvas size
        canvas.update()
        canvas_width = canvas.winfo_width()
        canvas_height = canvas.winfo_height()
        
        # Resize image to fit canvas while maintaining aspect ratio
        img_width, img_height = pil_image.size
        
        if canvas_width > 1 and canvas_height > 1:
            scale = min(canvas_width / img_width, canvas_height / img_height)
            new_width = int(img_width * scale)
            new_height = int(img_height * scale)
            
            pil_image = pil_image.resize((new_width, new_height), Image.Resampling.LANCZOS)
        
        # Convert to PhotoImage
        photo = ImageTk.PhotoImage(pil_image)
        
        # Display on canvas
        canvas.delete("all")
        canvas.create_image(canvas_width // 2, canvas_height // 2, image=photo, anchor=tk.CENTER)
        
        # Keep a reference to prevent garbage collection
        canvas.image = photo


def main():
    """Main entry point for the application."""
    root = tk.Tk()
    app = AnotoGUI(root)
    root.mainloop()


if __name__ == "__main__":
    main()
