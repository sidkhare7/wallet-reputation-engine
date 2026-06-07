# Trading Reputation Algorithm Analysis

This module provides comprehensive data analysis tools to validate and verify trading reputation algorithms for ranking users by trading volume, holding duration, holdings value, and other metrics.

## Installation

First, install the required Python packages:

```bash
pip3 install -r requirements.txt
```

Or install individually:
```bash
pip3 install pandas matplotlib seaborn scipy numpy
```

## Usage

### Quick Start

Run the complete analysis:

```bash
python3 analyze.py
```

### Custom Analysis

You can also use individual analysis functions:

```python
from analyze import TradingReputationAnalyzer

# Initialize analyzer
analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')

# Run specific analyses
analyzer.plot_reputation_distributions()
analyzer.plot_correlation_matrix()
analyzer.plot_volume_analysis()
analyzer.plot_holding_analysis()
analyzer.plot_reputation_ranking_validation()
analyzer.plot_outlier_detection()
analyzer.plot_temporal_analysis()
analyzer.generate_algorithm_report()
```

## Available Analyses

### 1. Distribution Analysis
- **Function**: `plot_reputation_distributions()`
- **Purpose**: Shows the distribution of key metrics (volume, holding duration, holdings value, diamond hand probability)
- **Helps identify**: Data skewness, outliers, and metric ranges

### 2. Correlation Analysis
- **Function**: `plot_correlation_matrix()`
- **Purpose**: Analyzes relationships between different reputation metrics
- **Helps identify**: Which metrics are related and potential redundancies

### 3. Volume Analysis
- **Function**: `plot_volume_analysis()`
- **Purpose**: Deep dive into trading volume patterns
- **Includes**: Buy vs sell volume, volume distributions, volume vs holdings correlation

### 4. Holding Pattern Analysis
- **Function**: `plot_holding_analysis()`
- **Purpose**: Analyzes holding duration and value patterns
- **Helps identify**: Diamond hand behavior and holding strategies

### 5. Ranking Algorithm Validation
- **Function**: `plot_reputation_ranking_validation()`
- **Purpose**: Validates the ranking algorithm effectiveness
- **Features**: 
  - Creates composite scores with adjustable weights
  - Compares top vs bottom performers
  - Shows score component contributions

### 6. Outlier Detection
- **Function**: `plot_outlier_detection()`
- **Purpose**: Identifies potential data quality issues or manipulation
- **Method**: Uses IQR (Interquartile Range) method for outlier detection

### 7. Temporal Analysis
- **Function**: `plot_temporal_analysis()`
- **Purpose**: Analyzes trading patterns over time
- **Includes**: Daily, hourly, and weekly activity patterns

## Algorithm Weights

The default composite reputation score uses these weights:
- Volume: 30%
- Holding Duration: 30% 
- Holdings Value: 25%
- Diamond Hand Probability: 15%

You can adjust these weights in the `plot_reputation_ranking_validation()` function.

## Data Requirements

The CSV file should contain these columns:
- `id`: User identifier
- `diamond_hand_probability`: Diamond hand score
- `volume`: Total trading volume
- `holding_duration`: How long tokens are held
- `holdings_value`: Current value of holdings
- `buy_volume`: Total buy volume
- `sell_volume`: Total sell volume
- `first_bought`: Timestamp of first purchase (optional, for temporal analysis)

## Output

The analysis generates:
1. **Multiple visualization charts** showing different aspects of the data
2. **Correlation insights** with strongest correlations highlighted
3. **Ranking validation** with top performer analysis
4. **Comprehensive report** with recommendations

## Troubleshooting

### Common Issues

1. **Missing packages**: Install all requirements using `pip3 install -r requirements.txt`
2. **Large CSV files**: The script handles large files efficiently, but very large datasets may need chunking
3. **No data for certain metrics**: Some charts may show "No Data" if all values are zero for a metric

### Performance Tips

- For very large datasets (>100K rows), consider sampling for visualization
- Log scales are automatically applied for metrics with wide ranges
- Charts are optimized for readability with appropriate binning

## Customization

You can easily customize the analysis by:
1. Adjusting weights in the ranking algorithm
2. Modifying chart sizes and styles
3. Adding new metrics to the analysis
4. Changing outlier detection thresholds

## Support

For issues or feature requests, please refer to the code comments or modify the functions as needed for your specific use case. 