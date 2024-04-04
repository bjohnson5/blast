import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
VISUALS_DIR = os.path.join(SCRIPT_DIR, '../visuals')

def create_pie_chart(df, filename):
    # Convert 'Flat%' column to numeric
    df['Flat%'] = df['Flat%'].str.rstrip('%').astype(float)
    
    # Filter data for time 4 and flat percentage > 3%
    filtered_data = df[(df['Time'] == 4) & (df['Flat%'] > 3)]
    
    if filtered_data.empty:
        print(f"No data found for time 4 with flat percentage > 3% in {filename}")
        return
    
    # Group by name and calculate total flat percentage
    grouped_data = filtered_data.groupby('Name')['Flat%'].sum().reset_index()
    
    # Prepare data for pie chart
    names = grouped_data['Name']
    flat_percentages = grouped_data['Flat%']
    
    plt.style.use('theme.mplstyle')
    dark_palette = ['#004c6d', '#136383', '#267c9a', '#3895af', '#4cafc4', '#62c9d9', '#79e4ec', '#93ffff']

    # Create pie chart
    plt.figure(figsize=(20, 8))
    plt.pie(flat_percentages, colors=dark_palette, labels=names, autopct='%1.1f%%', startangle=140)
    plt.axis('equal')  # Equal aspect ratio ensures that pie is drawn as a circle
    
    # Extract test case number from the filename
    test_case_number = filename.split('_')[-1].split('/')[0]

    # Add title to the pie chart
    plt.title(f"Function Memory Usage: Test Case {test_case_number}", loc='left', pad=20, fontsize=16, fontweight='bold')

    # Save pie chart
    chart_filename = os.path.join(VISUALS_DIR, f"test_case_{test_case_number}_pie_chart.png")
    plt.savefig(chart_filename, bbox_inches='tight')
    print(f"Saved pie chart as {chart_filename}")
    plt.close()

def create_function_data_dict(csv_files):
    function_data = {}
    
    # Loop through each CSV file
    for i, file in enumerate(csv_files, start=1):
        # Read CSV file into DataFrame
        df = pd.read_csv(file)
        
        # Filter data for time 0
        filtered_data = df[df['Time'] == 0]
        
        # Iterate over each row in the filtered data
        for index, row in filtered_data.iterrows():
            function_name = row['Name']
            flat_percentage = row['Flat%']
            
            # Convert flat percentage to float
            flat_percentage = float(flat_percentage.rstrip('%'))
            
            # Add function name to dictionary if not already present
            if function_name not in function_data:
                # Initialize list with zeros for previous test cases
                function_data[function_name] = [0] * (i - 1)
            
            # Set flat percentage for current test case
            function_data[function_name].append(flat_percentage)
    
    return function_data

def filter_function_data(function_data):
    # Filter dictionary to remove functions with flat percentages below 3% in all test cases
    function_data_filtered = {func_name: values for func_name, values in function_data.items() if any(value > 3 for value in values)}
    
    return function_data_filtered

def create_bar_chart(function_dictionary):
    tests = ("Test Case 11", "Test Case 12", "Test Case 13")
    x_labels = list(function_dictionary.keys())
    width = 0.25  # the width of the bars
    multiplier = 0

    fig, ax = plt.subplots()

    for test_index, test in enumerate(tests):
        offset = width * test_index
        values = [function_dictionary[x_label][test_index] for x_label in x_labels]
        rects = ax.bar(np.arange(len(x_labels)) + offset, values, width, label=test)
        ax.bar_label(rects, padding=3)
        multiplier += 1

    # Add some text for labels, title and custom x-axis tick labels, etc.
    ax.set_title('Memory by function')
    ax.set_xticks(np.arange(len(x_labels)) + width * (len(tests) - 1) / 2)
    ax.set_xticklabels(x_labels)
    ax.legend(loc='upper left', bbox_to_anchor=(1, 1))
    ax.set_ylim(0, 110)

    plt.show()

def plot_flat_values(function_name, run_number, csv_files):
    flat_values = []
    
    # Iterate over each CSV file
    for file in csv_files:
        # Read CSV file into DataFrame
        df = pd.read_csv(file)
        
        # Filter data for function name and run number
        filtered_data = df[(df['Name'] == function_name) & (df['Time'] == run_number)]
        
        # Check if data exists for the given function name and run number
        if not filtered_data.empty:
            # Extract flat value and convert to numerical value
            flat_value_str = filtered_data.iloc[0]['Flat']
            flat_value = float(flat_value_str[:-2])  # Remove 'MB' and convert to float
            flat_values.append(flat_value)
        else:
            # If no data found, append NaN
            flat_values.append(float('nan'))
    
    # Sort flat values in descending order
    flat_values.sort(reverse=True)
    
    # Test case labels
    test_cases = [f"Test Case {i}" for i in range(11, 14)]
    
    # Plot line graph
    plt.plot(test_cases, flat_values, marker='o')
    plt.title(f'Flat Value for {function_name}')
    plt.ylabel('Flat Value (MB)')
    plt.grid(True)
    
    # Add value labels above each point
    for i, value in enumerate(flat_values):
        plt.text(test_cases[i], value, f"{value:.2f}", ha='center', va='bottom', fontsize=9)
    
    # Save pie chart
    chart_filename = os.path.join(VISUALS_DIR, f"Line_Graph.png")
    plt.savefig(chart_filename, bbox_inches='tight')
    print(f"Saved Line Graph as {chart_filename}")
    plt.close()

def main():
    # Construct the paths to CSV files relative to the script directory
    csv_to_pie = [os.path.join(SCRIPT_DIR, '../', f'test_case_{i}', 'pprof.csv') for i in range(1, 11)]

    csv_to_line = [os.path.join(SCRIPT_DIR, '../', f'test_case_{i}', 'pprof.csv') for i in range(11, 14)]

    # Process each CSV file
    for file in csv_to_pie:
        try:
            # Read CSV file into DataFrame
            df = pd.read_csv(file)
            
            # Create pie chart for time 4
            create_pie_chart(df, file)
        except FileNotFoundError:
            print(f"File {file} not found.")

    # Read each CSV file into a DataFrame
    #function_dict = filter_function_data(create_function_data_dict(csv_to_bar))

    # Create bar chart for the list of DataFrames
    #create_bar_chart(function_dict)
            
    plot_flat_values("golang.org/x/crypto/scrypt.Key", 0, csv_to_line)

if __name__ == "__main__":
    main()
