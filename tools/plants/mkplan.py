import json
def plan(forest, edge, meadow, lake, cold):
    S = []
    S += [{"season": s, "name": "island", "x": 11, "y": 11, "bars": [0]} for s in ["summer", "autumn", "winter"]]
    S += [{"season": "summer", "name": "forest", "x": forest[0], "y": forest[1], "bars": [5, 10, 20, 50, 200, 2000], "angled": [10, 20]},
          {"season": "summer", "name": "forestT", "x": forest[0], "y": forest[1], "bars": [50], "views": ["true"]},
          {"season": "autumn", "name": "forest", "x": forest[0], "y": forest[1], "bars": [10, 50, 200, 2000], "angled": [10]},
          {"season": "winter", "name": "forest", "x": forest[0], "y": forest[1], "bars": [10, 50]},
          {"season": "summer", "name": "edge", "x": edge[0], "y": edge[1], "bars": [10, 50, 200]},
          {"season": "autumn", "name": "edge", "x": edge[0], "y": edge[1], "bars": [50]},
          {"season": "summer", "name": "meadow", "x": meadow[0], "y": meadow[1], "bars": [5, 10, 50], "angled": [10]},
          {"season": "autumn", "name": "meadow", "x": meadow[0], "y": meadow[1], "bars": [10]},
          {"season": "summer", "name": "lake", "x": lake[0], "y": lake[1], "bars": [10, 50]},
          {"season": "summer", "name": "cold", "x": cold[0], "y": cold[1], "bars": [10, 50]},
          {"season": "winter", "name": "cold", "x": cold[0], "y": cold[1], "bars": [10, 50]}]
    return {"seasons": {"winter": [15, 44], "summer": [196, 225], "autumn": [318, 347]}, "shots": S,
            "sweeps": [{"season": "summer", "name": "sweep", "x": forest[0], "y": forest[1], "steps": 8}]}
json.dump(plan((14.5, 5.5), (7.0, 6.5), (4.5, 6.6), (12.34, 10.74), (9.5, 9.5)), open('plan1.json', 'w'))
json.dump(plan((10.5, 12.5), (15.0, 11.5), (6.0, 15.8), (17.47, 9.6), (13.5, 12.5)), open('plan2.json', 'w'))
